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
        self.create_base_dir()
    }

    fn create_base_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.config.root_base_dir_path())?;
        Ok(())
    }
}
