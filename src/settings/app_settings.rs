use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    RwLock,
};

use crate::service::BaseService;

pub struct AppSettings<S> {
    save_path: PathBuf,
    event_sender: Sender<()>,
    settings: S,
}

#[cfg(windows)]
fn get_default_save_path() -> PathBuf {
    #[cfg(debug_assertions)]
    static RELATIVE_SAVE_PATH: &str =
        concat!(env!("CARGO_PKG_NAME"), "/settings-dbg.json");

    #[cfg(not(debug_assertions))]
    static RELATIVE_SAVE_PATH: &str =
        concat!(env!("CARGO_PKG_NAME"), "/settings.json");

    let app_data = std::env::var("APPDATA").expect("APPDATA should be present");
    Path::new(&app_data).join(RELATIVE_SAVE_PATH)
}

impl<S> AppSettings<S>
where
    S: Serialize + for<'de> Deserialize<'de> + Default + Send + Sync,
{
    pub fn default() -> Result<Arc<RwLock<Self>>> {
        let save_path = get_default_save_path();
        AppSettings::<S>::new(save_path)
    }

    pub fn new(save_path: impl Into<PathBuf>) -> Result<Arc<RwLock<Self>>> {
        let save_path = save_path.into();
        std::fs::create_dir_all(&save_path.parent().unwrap())?;
        let (tx, _) = channel(16);
        let settings = Arc::new(RwLock::new(AppSettings {
            save_path,
            event_sender: tx,
            settings: S::default(),
        }));
        Ok(settings)
    }

    pub fn get_settings(&self) -> &S {
        &self.settings
    }

    pub fn get_settings_mut(&mut self) -> &mut S {
        &mut self.settings
    }

    pub fn notify_settings_changed(&self) {
        let _ = self.event_sender.send(());
    }

    /// Writes the current settings to disk
    pub async fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.settings)?;
        tokio::fs::write(&self.save_path, json).await?;
        self.notify_settings_changed();
        Ok(())
    }

    /// Loads the settings from disk, overriding the currently loaded ones.
    /// Does nothing if the file doesn't exist.
    pub async fn load(&mut self) -> Result<()> {
        let file_contents = tokio::fs::read(&self.save_path).await;
        let file_contents = match file_contents {
            Ok(res) => res,
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => return Ok(()),
                _ => bail!(e),
            },
        };
        self.settings = serde_json::from_slice::<S>(&file_contents)?;
        self.notify_settings_changed();
        Ok(())
    }
}

impl<S> BaseService<()> for AppSettings<S>
where
    S: Send + Sync,
{
    fn subscribe(&self) -> Receiver<()> {
        self.event_sender.subscribe()
    }
}

#[cfg(test)]
mod test {
    use anyhow::ensure;
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use test_context::{test_context, AsyncTestContext};

    #[derive(Serialize, Deserialize, Default, PartialEq)]
    struct TestSettings {
        int: u32,
        hello: String,
        nice: bool,
    }

    use super::*;

    struct Context {
        path: PathBuf,
    }

    impl AsyncTestContext for Context {
        async fn setup() -> Self {
            let mut rng: StdRng = StdRng::from_os_rng();

            let dire =
                std::env::temp_dir().join(format!("spotick-test/{}-settings.json", rng.next_u64()));
            println!("Dire: {:?}", &dire);
            Self { path: dire }
        }

        async fn teardown(self) {
            let _ = std::fs::remove_file(self.path);
        }
    }

    #[test_context(Context)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn empty_settings(ctx: &mut Context) -> Result<()> {
        let app_settings = AppSettings::new(&ctx.path)?;
        app_settings.write().await.save().await?;
        let json = std::fs::read(&ctx.path)?;
        let settings = serde_json::from_slice::<TestSettings>(&json)?;
        ensure!(
            &settings == app_settings.read().await.get_settings(),
            "Default settings differ"
        );
        Ok(())
    }

    #[test]
    fn correct_default_save_path() {
        std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        let default_path = get_default_save_path();
        assert_eq!(
            default_path,
            PathBuf::from("C:\\Users\\test\\AppData\\Roaming\\spotick\\settings.json")
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn load_non_existing() -> Result<()> {
        let app_settings = AppSettings::<TestSettings>::new("test.json")?;
        app_settings.write().await.load().await?;
        Ok(())
    }

    #[test_context(Context)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn simple_setting(ctx: &mut Context) -> Result<()> {
        {
            let settings = AppSettings::<TestSettings>::new(&ctx.path)?;
            settings.write().await.get_settings_mut().nice = true;
            settings.write().await.get_settings_mut().hello = "world".into();
            settings.write().await.save().await?;
        }

        let settings = AppSettings::<TestSettings>::new(&ctx.path)?;
        settings.write().await.load().await?;
        ensure!(settings.read().await.get_settings().nice, "Expected true");
        ensure!(
            &settings.read().await.get_settings().hello == "world",
            "Expected true"
        );
        Ok(())
    }
}
