use std::{
    io::Cursor,
    num::NonZero,
    sync::{Arc, Weak},
};

use anyhow::{ensure, Result};
use image::ImageReader;
use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    RwLock,
};
use windows::{
    core::{Result as WinResult, HSTRING},
    Foundation::TypedEventHandler,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
    },
    Storage::Streams::{DataReader, IRandomAccessStreamReference, InputStreamOptions},
};

use crate::service::{
    media_service::{
        AlbumCover, MediaService, MediaServiceError, MediaTrack, PlaybackChangedEvent,
        PlaybackState,
    },
    BaseService,
};

type WinRtHandle = Option<NonZero<i64>>;

/// A media service observing one running application connected to
/// the media controls of the windows runtime (winrt).
/// NOTE: The winrt media API doesn't support individual media volume
/// (i.e. getting or requesting the monitored app to change its volume).
/// Seeking into and reporting the playback position is currently a Todo.
pub struct WindowsMediaService {
    self_ref: Weak<RwLock<WindowsMediaService>>,
    manager: GlobalSystemMediaTransportControlsSessionManager,
    sessions_changed_handler: WinRtHandle,
    source_app_id: String,
    media_properties_changed_handler: WinRtHandle,
    media_playback_changed_handler: WinRtHandle,
    source_session: Option<GlobalSystemMediaTransportControlsSession>,
    current_track: Option<MediaTrack>,
    playback_state: PlaybackState,
    event_sender: Sender<PlaybackChangedEvent>,
}

fn unwrap_hstring(hstring: WinResult<HSTRING>, default: impl Into<String>) -> String {
    hstring
        .ok()
        .and_then(|s| {
            if !s.is_empty() {
                Some(s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            log::warn!("Could not retrieve HSTRING");
            default.into()
        })
}

fn convert_ticks_to_seconds(ticks: i64) -> u64 {
    if ticks < 0 {
        return 0;
    }

    ticks as u64 / 10_000_000
}

macro_rules! register_winrt_event {
    ($self:ident, $src:expr, $ev:ident, |$srv:ident|$handler:block) => {{
        $src.$ev(&TypedEventHandler::new({
            let srv = $self.clone();
            let rt_handle = tokio::runtime::Handle::current();
            move |_, __| {
                let srv = srv.clone();
                rt_handle.spawn(async move {
                    log::info!(stringify!($ev));
                    if let Some($srv) = srv.upgrade() {
                        let res: Result<(), MediaServiceError> = $handler;
                        if let Err(e) = res {
                            log::error!("WinRt handler failed: {:?}", e);
                        }
                    } else {
                        log::error!("Could not get service in winrt handler!");
                    }
                });
                Ok(())
            }
        }))
    }};
}

impl WindowsMediaService {
    /// Creates a new media service monitoring the application identified by
    /// the [source_app_id] (usually the application image name - i.e. file name).
    ///
    /// To monitor Spotify for example:
    /// ```
    /// let srv = WindowsMediaService::new("Spotify.exe");
    /// srv.write().await.begin_monitor_sessions()?;
    /// ```
    ///
    /// You have to call [WindowsMediaService::begin_monitor_sessions] to receive
    /// [PlaybackChangedEvent]s.
    pub fn new(source_app_id: impl Into<String>) -> Arc<RwLock<Self>> {
        Arc::new_cyclic(|weak| {
            let (tx, _) = channel(16);
            RwLock::new(WindowsMediaService {
                self_ref: weak.clone(),
                manager: GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                    .unwrap()
                    .get()
                    .unwrap(),
                sessions_changed_handler: None,
                media_properties_changed_handler: None,
                media_playback_changed_handler: None,
                source_session: None,
                current_track: None,
                playback_state: PlaybackState::default(),
                source_app_id: source_app_id.into().to_lowercase(),
                event_sender: tx,
            })
        })
    }

    fn send_event(&self, ev: PlaybackChangedEvent) {
        match ev {
            PlaybackChangedEvent::TrackChanged => {
                log::info!("{:?}: {:?}", ev, self.current_track);
            }
            PlaybackChangedEvent::Pause | PlaybackChangedEvent::Play => {
                log::info!("{:?}: {:?}", ev, self.playback_state);
            }
            _ => {}
        };
        let _ = self.event_sender.send(ev);
    }

    /// Starts monitoring for the media session identified by its source app id.
    /// Does nothing if already started.
    pub fn begin_monitor_sessions(&mut self) -> Result<(), MediaServiceError> {
        if self.sessions_changed_handler.is_some() {
            return Ok(());
        }

        self.update_sessions()?;
        let handle = register_winrt_event!(self, self.manager, SessionsChanged, |srv| {
            srv.write().await.update_sessions()
        })?;
        self.sessions_changed_handler = NonZero::new(handle);
        Ok(())
    }

    fn begin_monitor_source_session(&mut self) -> Result<(), MediaServiceError> {
        if self.media_properties_changed_handler.is_some()
            || self.media_playback_changed_handler.is_some()
        {
            return Ok(());
        }
        let Some(session) = &self.source_session else {
            return Ok(());
        };

        log::info!(
            "Beginning to monitor source session: {}",
            &self.source_app_id
        );

        let handle = register_winrt_event!(self, session, MediaPropertiesChanged, |srv| {
            srv.write().await.update_current_session_info()
        })?;
        self.media_properties_changed_handler = NonZero::new(handle);

        let handle = register_winrt_event!(self, session, PlaybackInfoChanged, |srv| {
            srv.write().await.update_playback_info()
        })?;
        self.media_playback_changed_handler = NonZero::new(handle);

        self.update_current_session_info()?;
        self.update_playback_info()?;

        Ok(())
    }

    fn update_sessions(&mut self) -> Result<(), MediaServiceError> {
        for session in self.manager.GetSessions()? {
            log::debug!("Found source with id: {}", session.SourceAppUserModelId()?);
            if session.SourceAppUserModelId()?.to_string().to_lowercase() == self.source_app_id {
                if self.source_session.is_none() {
                    self.source_session = Some(session);
                    self.begin_monitor_source_session()?;
                }
                return Ok(());
            }
        }
        self.end_monitor_source_session();
        self.source_session = None;
        Ok(())
    }

    fn update_current_session_info(&mut self) -> Result<(), MediaServiceError> {
        let Some(session) = &self.source_session else {
            return Ok(());
        };

        let media_props = session.TryGetMediaPropertiesAsync()?.get()?;
        let timeline_props = session.GetTimelineProperties()?;

        let title_length = convert_ticks_to_seconds(timeline_props.MaxSeekTime()?.Duration);
        let track = if title_length > 0 {
            let album_cover = match media_props.Thumbnail() {
                Ok(s) => match WindowsMediaService::read_thumbnail(s) {
                    Ok(cover) => cover,
                    Err(e) => {
                        log::error!("Unable to fetch thumbnail: {}", e);
                        AlbumCover::None
                    }
                },
                Err(_) => AlbumCover::None,
            };

            Some(MediaTrack {
                album_title: unwrap_hstring(media_props.AlbumTitle(), "No Title"),
                artist: unwrap_hstring(media_props.Artist(), "No Artist"),
                title: unwrap_hstring(media_props.Title(), "No Title"),
                length: title_length,
                album_cover,
            })
        } else {
            // We have no track
            None
        };

        self.current_track = track;
        self.send_event(PlaybackChangedEvent::TrackChanged);
        Ok(())
    }

    fn update_playback_info(&mut self) -> Result<(), MediaServiceError> {
        let Some(session) = &self.source_session else {
            return Ok(());
        };

        let playback = session.GetPlaybackInfo()?;
        // See: https://learn.microsoft.com/en-US/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionplaybackstatus?view=winrt-22621
        let playing = playback.PlaybackStatus()?.0 == 4;
        self.playback_state.is_playing = playing;
        self.send_event(if playing {
            PlaybackChangedEvent::Play
        } else {
            PlaybackChangedEvent::Pause
        });
        Ok(())
    }

    fn read_thumbnail(stream: IRandomAccessStreamReference) -> Result<AlbumCover> {
        let stream_handle = stream.OpenReadAsync()?.get()?;
        ensure!(stream_handle.CanRead()?, "Thumbnail is not readable");

        let buffer_size = stream_handle.Size()? as u32;
        log::info!(
            "Media thumbnail content-type: {}, Size: {}",
            stream_handle.ContentType()?,
            buffer_size
        );

        let buf_reader = DataReader::CreateDataReader(&stream_handle)?;
        buf_reader.SetInputStreamOptions(InputStreamOptions(2))?;
        buf_reader.LoadAsync(buffer_size)?.get()?;

        let mut buffer: Vec<u8> = Vec::with_capacity(buffer_size as usize);
        buffer.resize(buffer.capacity(), 0); // DataReader needs length == capacity
        buf_reader.ReadBytes(&mut buffer)?;

        let img_reader = ImageReader::new(Cursor::new(buffer)).with_guessed_format()?;
        let img = img_reader.decode()?.to_rgba8();

        Ok(AlbumCover::Image(img))
    }

    /// Stops monitoring for the source media session.
    /// Does nothing if not already monitored.
    /// Subscribers won't receive events after this call.
    pub fn end_monitor_sessions(&mut self) {
        log::info!("Stopping monitoring media sessions");
        if let Some(handle) = self.sessions_changed_handler.take() {
            let _ = self.manager.RemoveSessionsChanged(handle.get());
        }
    }

    fn end_monitor_source_session(&mut self) {
        log::info!("Stopping monitoring source media session");
        if let Some(session) = self.source_session.take() {
            if let Some(handle) = self.media_properties_changed_handler.take() {
                let _ = session.RemoveMediaPropertiesChanged(handle.get());
            }
            if let Some(handle) = self.media_playback_changed_handler.take() {
                let _ = session.RemovePlaybackInfoChanged(handle.get());
            }
        }
        self.current_track = None;
        self.send_event(PlaybackChangedEvent::TrackChanged);
    }

    pub fn clone(&self) -> Weak<RwLock<Self>> {
        self.self_ref.clone()
    }
}

impl Drop for WindowsMediaService {
    fn drop(&mut self) {
        self.end_monitor_sessions();
        self.end_monitor_source_session();
    }
}

impl BaseService<PlaybackChangedEvent> for WindowsMediaService {
    fn subscribe(&self) -> Receiver<PlaybackChangedEvent> {
        self.event_sender.subscribe()
    }
}

macro_rules! wait_async_op {
    ($async_op:expr) => {
        let x = $async_op;
        tokio::task::spawn_blocking(move || x.get())
            .await
            .unwrap()?
    };
}

#[async_trait::async_trait]
impl MediaService for WindowsMediaService {
    async fn next_track(&mut self) -> Result<(), MediaServiceError> {
        if let Some(session) = &self.source_session {
            wait_async_op!(session.TrySkipNextAsync()?);
        }
        Ok(())
    }

    async fn previous_track(&mut self) -> Result<(), MediaServiceError> {
        if let Some(session) = &self.source_session {
            wait_async_op!(session.TrySkipPreviousAsync()?);
        }
        Ok(())
    }

    async fn play(&mut self) -> Result<(), MediaServiceError> {
        if let Some(session) = &self.source_session {
            wait_async_op!(session.TryPlayAsync()?);
        }
        Ok(())
    }

    async fn pause(&mut self) -> Result<(), MediaServiceError> {
        if let Some(session) = &self.source_session {
            wait_async_op!(session.TryPauseAsync()?);
        }
        Ok(())
    }

    async fn seek(&mut self, _playback_percent: u32) -> Result<(), MediaServiceError> {
        Ok(())
    }

    async fn set_volume(&mut self, _volume: u32) -> Result<(), MediaServiceError> {
        Ok(())
    }

    fn current_track(&self) -> Option<&MediaTrack> {
        self.current_track.as_ref()
    }

    fn current_playback_state(&self) -> &PlaybackState {
        &self.playback_state
    }
}
