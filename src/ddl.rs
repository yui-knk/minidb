use std::fs;
use std::io::{self, Error, ErrorKind};
use config::Config;
use catalog::catalog::RecordManeger;
use catalog::mini_database::MiniDatabaseRecord;

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

    pub fn execute(&self, dbname: &str) -> io::Result<()> {
        self.check_base_dir()?;
        self.create_database_dir(dbname)?;
        self.add_record(dbname);
        Ok(())
    }

    fn check_base_dir(&self) -> io::Result<()> {
        if self.config.base_dir_path().exists() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("Base dir ({}) does not exist.", self.config.base_dir_path().display())
            ))
        }
    }

    fn create_database_dir(&self, dbname: &str) -> io::Result<()> {
        fs::create_dir(self.config.database_dir_path(dbname))
    }

    fn add_record(&self, dbname: &str) {
        let mut db: RecordManeger<MiniDatabaseRecord> = RecordManeger::build_from_config("mini_database".to_string(), &self.config).unwrap();
        let record = MiniDatabaseRecord::new(dbname.to_string());
        db.add_record(record);
        db.save(&self.config);
    }
}

impl CreateTableCommand {
    pub fn new(config: Config) -> CreateTableCommand {
        CreateTableCommand { config: config }
    }

    pub fn execute(&self, dbname: &str, tablename: &str) -> io::Result<()> {
        self.check_base_dir()?;
        self.create_table_dir(dbname, tablename)
    }

    fn check_base_dir(&self) -> io::Result<()> {
        if self.config.base_dir_path().exists() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Other,
                format!("Base dir ({}) does not exist.", self.config.base_dir_path().display())
            ))
        }
    }

    fn create_table_dir(&self, dbname: &str, tablename: &str) -> io::Result<()> {
        fs::create_dir(self.config.table_dir_path(dbname, tablename))
    }
}
