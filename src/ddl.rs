use std::fs;
use std::io::{Error, ErrorKind};
use config::Config;

pub struct CreateDatabaseCommand {
    config: Config,
}

pub struct CreateTableCommand {
    config: Config,
}

impl CreateDatabaseCommand {
    pub fn new(config: Config) -> CreateDatabaseCommand {
        CreateDatabaseCommand { config: config }
    }

    pub fn execute(&self, dbname: String) -> std::io::Result<()> {
        self.check_base_dir()?;
        self.create_database_dir(dbname)
    }

    fn check_base_dir(&self) -> std::io::Result<()> {
        if self.config.base_dir_path().exists() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("Base dir ({}) does not exist.", self.config.base_dir_path().display())
            ))
        }
    }

    fn create_database_dir(&self, dbname: String) -> std::io::Result<()> {
        fs::create_dir(self.config.database_dir_path(dbname))
    }
}

impl CreateTableCommand {
    pub fn new(config: Config) -> CreateTableCommand {
        CreateTableCommand { config: config }
    }

    pub fn execute(&self, dbname: String, tablename: String) -> std::io::Result<()> {
        self.check_base_dir()?;
        self.create_table_dir(dbname, tablename)
    }

    fn check_base_dir(&self) -> std::io::Result<()> {
        if self.config.base_dir_path().exists() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("Base dir ({}) does not exist.", self.config.base_dir_path().display())
            ))
        }
    }

    fn create_table_dir(&self, dbname: String, tablename: String) -> std::io::Result<()> {
        fs::create_dir(self.config.table_dir_path(dbname, tablename))
    }
}
