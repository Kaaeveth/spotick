use tokio::sync::broadcast::Receiver;

pub use crate::service::windows_media_service::WindowsMediaService;
pub use crate::service::media_service::{MediaService, PlaybackChangedEvent, SharedMediaService};

mod media_service;
mod windows_media_service;

pub trait BaseService<E: Clone>: Send + Sync {
    fn subscribe(&self) -> Receiver<E>;
}
