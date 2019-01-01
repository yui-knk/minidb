use std::fs;
use config::Config;

pub struct InitCommand {
    config: Config,
}

impl InitCommand {
    pub fn new(config: Config) -> InitCommand {
        InitCommand { config: config }
    }

    pub fn execute(&self) -> std::io::Result<()> {
        self.create_base_dir()?;
        self.create_global_dir()
    }

    fn create_base_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.config.base_dir_path())?;
        Ok(())
    }

    fn create_global_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.config.global_dir_path())?;
        Ok(())
    }
}
