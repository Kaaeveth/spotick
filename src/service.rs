use std::sync::Weak;

use tokio::sync::{broadcast::Receiver, RwLock};

pub use crate::service::windows_media_service::WindowsMediaService;
pub use crate::service::media_service::{MediaService, PlaybackChangedEvent};

mod media_service;
mod windows_media_service;


#[derive(Clone)]
pub struct ServiceEvent<E: Clone> {
    pub sender: Weak<RwLock<dyn BaseService<E>>>,
    pub event: E
}

pub trait BaseService<E: Clone>: Send + Sync {
    fn subscribe(&self) -> Receiver<ServiceEvent<E>>;
}
