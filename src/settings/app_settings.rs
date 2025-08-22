use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use anyhow::Result;

pub struct AppSettings<S> {
    save_path: PathBuf,
    settings: S
}

#[cfg(windows)]
fn get_default_save_path() -> PathBuf {
    use std::path::Path;

    static RELATIVE_SAVE_PATH: &'static str = concat!(env!("CARGO_PKG_NAME"), "/settings.json");
    let app_data = std::env::var("APPDATA").expect("APPDATA should be present");

    Path::new(&app_data).join(RELATIVE_SAVE_PATH)
}

impl<S> AppSettings<S>
where 
    S: Serialize + for<'de> Deserialize<'de> + Default + 'static
{
    pub fn default() -> Result<Self> {
        let save_path = get_default_save_path();
        AppSettings::<S>::new(save_path)
    }

    pub fn new(save_path: impl Into<PathBuf>) -> Result<Self> {
        let save_path = save_path.into();
        std::fs::create_dir_all(&save_path.parent().unwrap())?;
        Ok(AppSettings {
            save_path,
            settings: S::default()
        })
    }

    pub fn get_settings(&self) -> &S {
        &self.settings
    }

    pub fn get_settings_mut(&mut self) -> &mut S {
        &mut self.settings
    }

    /// Writes the current settings to disk
    pub async fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.settings)?;
        tokio::fs::write(&self.save_path, json).await?;
        Ok(())
    }

    /// Loads the settings from disk, overriding the currently loaded ones.
    pub async fn load(&mut self) -> Result<()> {
        let file_contents = tokio::fs::read(&self.save_path).await?;
        self.settings = serde_json::from_slice::<S>(&file_contents)?;
        Ok(())
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
        nice: bool
    }

    use super::*;

    struct Context {
        path: PathBuf
    }

    impl AsyncTestContext for Context {
        async fn setup() -> Self {
            let mut rng: StdRng = StdRng::from_os_rng();
        
            let dire = std::env::temp_dir().join(format!("spotick-test/{}-settings.json", rng.next_u64()));
            println!("Dire: {:?}", &dire);
            Self {
                path: dire
            }
        }

        async fn teardown(self) {
            let _ = std::fs::remove_file(self.path);
        }
    }

    #[test_context(Context)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn empty_settings(ctx: &mut Context) -> Result<()> {
        let app_settings = AppSettings::new(&ctx.path)?;
        app_settings.save().await?;
        let json = std::fs::read(&ctx.path)?;
        let settings = serde_json::from_slice::<TestSettings>(&json)?;
        ensure!(&settings == app_settings.get_settings(), "Default settings differ");
        Ok(())
    }

    #[test]
    fn correct_default_save_path() {
        std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        let default_path = get_default_save_path();
        assert_eq!(default_path, PathBuf::from("C:\\Users\\test\\AppData\\Roaming\\spotick\\settings.json"));
    }

    #[test_context(Context)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn simple_setting(ctx: &mut Context) -> Result<()> {
        {
            let mut settings = AppSettings::<TestSettings>::new(&ctx.path)?;
            settings.get_settings_mut().nice = true;
            settings.get_settings_mut().hello = "world".into();
            settings.save().await?;
        }

        let mut settings = AppSettings::<TestSettings>::new(&ctx.path)?;
        settings.load().await?;
        ensure!(settings.get_settings().nice, "Expected true");
        ensure!(&settings.get_settings().hello == "world", "Expected true");
        Ok(())
    }
}
