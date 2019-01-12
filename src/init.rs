use std::fs::{self, File};
use std::rc::Rc;

use config::Config;

pub struct InitCommand {
    config: Rc<Config>,
}

impl InitCommand {
    pub fn new(config: Rc<Config>) -> InitCommand {
        InitCommand { config: config }
    }

    pub fn execute(&self) -> std::io::Result<()> {
        self.create_base_dir()?;
        self.create_global_dir()?;
        self.system_catalog_dir_and_file("mini_database")?;
        // For tables
        self.system_catalog_dir_and_file("mini_class")?;
        // For columns
        self.system_catalog_dir_and_file("mini_attribute")
    }

    fn create_base_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.config.base_dir_path())
    }

    fn create_global_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.config.global_dir_path())
    }

    fn system_catalog_dir_and_file(&self, tablename: &str) -> std::io::Result<()> {
        fs::create_dir(self.config.system_catalog_dir_path(tablename))?;
        File::create(self.config.system_catalog_file_path(tablename))?;
        Ok(())
    }
}
