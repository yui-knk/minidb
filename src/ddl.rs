use std::fs;
use std::io::{self, Error, ErrorKind};
use std::rc::Rc;
use std::sync::RwLock;

use config::Config;
use catalog::catalog::RecordManeger;
use catalog::mini_class::MiniClassRecord;
use catalog::mini_database::MiniDatabaseRecord;
use catalog::mini_attribute::{MiniAttributeRecord, TypeLabel};
use oid_manager::{OidManager, Oid};

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
        let oid = self.oid_manager.write().unwrap().get_new_oid();
        self.check_base_dir()?;
        self.create_database_dir(oid)?;
        self.add_record(dbname, oid);
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

    fn create_database_dir(&self, db_oid: Oid) -> io::Result<()> {
        fs::create_dir(self.config.database_dir_path(db_oid))
    }

    fn add_record(&self, dbname: &str, db_oid: Oid) {
        let mut db: RecordManeger<MiniDatabaseRecord> = RecordManeger::mini_database_rm(&self.config);
        let record = MiniDatabaseRecord::new(db_oid, dbname.to_string());
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
        let db: RecordManeger<MiniDatabaseRecord> = RecordManeger::mini_database_rm(&self.config);
        let db_oid = db.find_mini_database(dbname).expect(&format!("{} database should be defined.", dbname)).oid;
        let table_oid = self.oid_manager.write().unwrap().get_new_oid();

        self.check_base_dir()?;
        self.create_table_dir(db_oid, table_oid)?;
        self.add_record_to_mini_class(dbname, tablename, table_oid);
        self.add_record_to_mini_attribute("id", db_oid, table_oid, TypeLabel::Integer, 4);
        self.add_record_to_mini_attribute("age", db_oid, table_oid, TypeLabel::Integer, 4);
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

    fn create_table_dir(&self, db_oid: Oid, table_oid: Oid) -> io::Result<()> {
        fs::create_dir(self.config.table_dir_path(db_oid, table_oid))
    }

    fn add_record_to_mini_class(&self, dbname: &str, tablename: &str, oid: Oid) {
        let mut db: RecordManeger<MiniClassRecord> = RecordManeger::mini_class_rm(&self.config);
        let record = MiniClassRecord::new(oid, tablename.to_string(), dbname.to_string());
        db.add_record(record);
        db.save(&self.config);
    }

    fn add_record_to_mini_attribute(&self, name: &str, db_oid: Oid, table_oid: Oid, ty: TypeLabel, len: usize) {
        let mut db: RecordManeger<MiniAttributeRecord> = RecordManeger::mini_attribute_rm(&self.config);
        let record = MiniAttributeRecord::new(
            name.to_string(),
            db_oid,
            table_oid,
            ty,
            len,
        );
        db.add_record(record);
        db.save(&self.config);
    }
}
