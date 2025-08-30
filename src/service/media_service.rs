#![allow(dead_code)]
use std::{fmt::Debug, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::service::BaseService;

#[derive(Clone, Debug)]
pub enum PlaybackChangedEvent {
    TrackChanged,
    Play,
    Pause,
    Volume,
    PlaybackProgress,
}

pub enum AlbumCover {
    Url(String),
    Image(image::RgbaImage),
    None,
}

impl AlbumCover {
    pub fn is_none(&self) -> bool {
        match self {
            AlbumCover::None => true,
            _ => false,
        }
    }
}

impl Debug for AlbumCover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let img_type = match self {
            AlbumCover::Image(_) => "RgbaImage",
            AlbumCover::None => "None",
            AlbumCover::Url(_) => "Url",
        };
        write!(f, "Image ({})", img_type)
    }
}

#[derive(Debug)]
pub struct MediaTrack {
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub album_cover: AlbumCover,
    pub length: u64, // seconds
}

#[derive(Default, Debug)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub volume: u32,           // %
    pub progress: Option<u32>, // %
}

#[derive(thiserror::Error, Debug)]
pub enum MediaServiceError {
    #[error("WinRT error")]
    WinRt(#[from] windows::core::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type SharedMediaService = Arc<RwLock<dyn MediaService>>;

#[async_trait]
/// Represents a (possibly remote) media player.
/// All methods returning a [anyhow::Result] may fail if the underlying player
/// is not available (e.g. not reachable, unauthorized, etc.).
pub trait MediaService: BaseService<PlaybackChangedEvent> {
    /// Attempts to play the next track if the underlying media service has one.
    /// If there is none, then [MediaService::current_track] will return [None].
    async fn next_track(&mut self) -> Result<(), MediaServiceError>;

    /// Attempts to play the previous track if the underlying media service has one.
    /// If there is none, then [MediaService::current_track] will return [None].
    async fn previous_track(&mut self) -> Result<(), MediaServiceError>;

    /// Attempts to play or resume the current [MediaTrack].
    /// Even if their is none, the concrete service may still select and play one.
    /// Does **not** fail if no track is available to be played.
    async fn play(&mut self) -> Result<(), MediaServiceError>;

    /// Pauses a currently running [MediaTrack], if there is one.
    /// Does nothing if nothing is playing.
    async fn pause(&mut self) -> Result<(), MediaServiceError>;

    /// Seeks into the current [MediaTrack] and resumes playback if paused.
    /// [playback_percent] must be between 0 and 100 (inclusive) and will be clamped otherwise.
    /// Does nothing if nothing is playing.
    async fn seek(&mut self, playback_percent: u32) -> Result<(), MediaServiceError>;

    /// Sets the volume of the underlying player.
    /// [volume] must be between 0 and 100 (inclusive) and will be clamped otherwise.
    async fn set_volume(&mut self, volume: u32) -> Result<(), MediaServiceError>;

    /// Sets the id of the media application to be controled and observed for changes.
    /// This id is platform dependent.
    /// On Windows, for example, it is the name of the application executable.
    fn set_source_app_id(&mut self, app_id: String) -> Result<(), MediaServiceError>;

    /// Gets the id of the currently controled and observed media application.
    /// See [MediaService::set_source_app_id] for more.
    fn get_source_app_id(&self) -> &str;

    fn current_track(&self) -> Option<&MediaTrack>;
    fn current_playback_state(&self) -> &PlaybackState;

    async fn toggle_playback(&mut self) -> Result<(), MediaServiceError> {
        let playback_state = self.current_playback_state();
        if playback_state.is_playing {
            self.pause().await?;
        } else {
            self.play().await?;
        }
        Ok(())
    }
}
