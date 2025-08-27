use tokio::sync::broadcast::Receiver;

pub use crate::service::media_service::{AlbumCover, PlaybackChangedEvent, SharedMediaService};
pub use crate::service::windows_media_service::WindowsMediaService;

mod media_service;
mod windows_media_service;

pub trait BaseService<E: Clone>: Send + Sync {
    fn subscribe(&self) -> Receiver<E>;
}
