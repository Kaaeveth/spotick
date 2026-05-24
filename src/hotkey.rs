use std::sync::{LazyLock, OnceLock};

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tokio::sync::broadcast::{channel, Receiver, Sender};

static SPOTICK_HOTKEYS_INSTANCE: OnceLock<HotkeyManager> = OnceLock::new();

// Hotkeys we want to listen for
// They need to be added below to handle_event as well
//
// Phantom-Key (Semi-Transparency)
static PHANTOM_KEY: LazyLock<HotKey> =
    LazyLock::new(|| HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space));

type Pressed = bool;
#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    PhantomKey(Pressed),
}

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    event_sender: Sender<HotkeyEvent>,
}

// SAFETY: The raw pointer (in GlobalHotKeyManager) is initialized once and never mutated after that.
unsafe impl Send for HotkeyManager {}
unsafe impl Sync for HotkeyManager {}

impl HotkeyManager {
    pub fn setup() {
        if std::thread::current().name().unwrap_or_default() != "main" {
            panic!("HotkeyManager needs to be setup from the main thread");
        }

        SPOTICK_HOTKEYS_INSTANCE.get_or_init(|| {
            let manager = GlobalHotKeyManager::new().unwrap();
            let sender = channel(8).0;
            let hk_manager = HotkeyManager {
                manager,
                event_sender: sender,
            };
            hk_manager.register_all_hotkeys();

            let sender = hk_manager.event_sender.clone();
            GlobalHotKeyEvent::set_event_handler(Some(move |ev| {
                HotkeyManager::handle_event(ev, &sender);
            }));

            hk_manager
        });
    }

    fn register_all_hotkeys(&self) {
        self.manager.register(*PHANTOM_KEY).unwrap();
    }

    /// Handles Hotkeys-Events and broadcasts them accordingly
    fn handle_event(ev: GlobalHotKeyEvent, sender: &Sender<HotkeyEvent>) {
        // Add any new Hotkeys here
        if ev.id == PHANTOM_KEY.id {
            let _ = sender.send(HotkeyEvent::PhantomKey(ev.state == HotKeyState::Pressed));
        }
    }

    pub fn subscribe(&self) -> Receiver<HotkeyEvent> {
        self.event_sender.subscribe()
    }

    pub fn get() -> &'static HotkeyManager {
        SPOTICK_HOTKEYS_INSTANCE
            .get()
            .unwrap_or_else(|| panic!("HotkeyManager has not been initialized"))
    }
}

#[macro_export]
macro_rules! listen_hotkeys {
    (|$hk:ident|$handler:block) => {{
        let mut hk_rv = HotkeyManager::get().subscribe();
        tokio::spawn(async move {
           loop {
               if let Ok($hk) = hk_rv.recv().await {
                   $handler
               } else {
                   break;
               }
           }
        });
    }};
}
