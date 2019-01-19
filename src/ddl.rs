use std::fs;
use std::io::{self, Error, ErrorKind};
use std::rc::Rc;
use std::sync::RwLock;

use config::Config;
use catalog::catalog::RecordManeger;
use catalog::mini_class::MiniClassRecord;
use catalog::mini_database::MiniDatabaseRecord;
use catalog::mini_attribute::{MiniAttributeRecord, TypeLabel};
use oid_manager::OidManager;

pub struct CreateDatabaseCommand {
    config: Rc<Config>,
    oid_manager: RwLock<OidManager>,
}

pub struct CreateTableCommand {
    config: Rc<Config>,
    oid_manager: RwLock<OidManager>,
}

impl CreateDatabaseCommand {
    pub fn new(config: Rc<Config>, oid_manager: RwLock<OidManager>) -> CreateDatabaseCommand {
        CreateDatabaseCommand {
            config: config,
            oid_manager: oid_manager,
        }
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
        let mut db: RecordManeger<MiniDatabaseRecord> = RecordManeger::mini_database_rm(&self.config);
        let oid = self.oid_manager.write().unwrap().get_new_oid();
        let record = MiniDatabaseRecord::new(oid, dbname.to_string());
        db.add_record(record);
        db.save(&self.config);
    }
}

impl CreateTableCommand {
    pub fn new(config: Rc<Config>, oid_manager: RwLock<OidManager>) -> CreateTableCommand {
        CreateTableCommand {
            config: config,
            oid_manager: oid_manager,
        }
    }

    pub fn execute(&self, dbname: &str, tablename: &str) -> io::Result<()> {
        self.check_base_dir()?;
        self.create_table_dir(dbname, tablename)?;
        self.add_record_to_mini_class(dbname, tablename);
        self.add_record_to_mini_attribute("id", dbname, tablename, TypeLabel::Integer, 4);
        self.add_record_to_mini_attribute("age", dbname, tablename, TypeLabel::Integer, 4);
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

    fn create_table_dir(&self, dbname: &str, tablename: &str) -> io::Result<()> {
        fs::create_dir(self.config.table_dir_path(dbname, tablename))
    }

    fn add_record_to_mini_class(&self, dbname: &str, tablename: &str) {
        let mut db: RecordManeger<MiniClassRecord> = RecordManeger::mini_class_rm(&self.config);
        let oid = self.oid_manager.write().unwrap().get_new_oid();
        let record = MiniClassRecord::new(oid, tablename.to_string(), dbname.to_string());
        db.add_record(record);
        db.save(&self.config);
    }

    fn add_record_to_mini_attribute(&self, name: &str, dbname: &str, tablename: &str, ty: TypeLabel, len: usize) {
        let mut db: RecordManeger<MiniAttributeRecord> = RecordManeger::mini_attribute_rm(&self.config);
        let record = MiniAttributeRecord::new(
            name.to_string(),
            dbname.to_string(),
            tablename.to_string(),
            ty,
            len,
        );
        db.add_record(record);
        db.save(&self.config);
    }
}
