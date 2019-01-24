use std::rc::Rc;

use config::{Config};
use catalog::catalog::RecordManeger;
use catalog::mini_database::MiniDatabaseRecord;
use catalog::mini_class::MiniClassRecord;
use catalog::mini_attribute::MiniAttributeRecord;
use tuple::{TupleTableSlot, KeyValue};
use buffer_manager::{BufferManager};
use storage_manager::{RelationManager};
use node_seqscan::{ScanState};
use node_insert::{InsertState};

pub struct InsertIntoCommnad {
    config: Rc<Config>,
}

pub struct SelectFromCommnad {
    config: Rc<Config>,
}

impl InsertIntoCommnad {
    pub fn new(config: Rc<Config>) -> InsertIntoCommnad {
        InsertIntoCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, key_values: Vec<KeyValue>) -> Result<(), String> {
        let db: RecordManeger<MiniDatabaseRecord> = RecordManeger::mini_database_rm(&self.config);
        let db_oid = db.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));
        let table: RecordManeger<MiniClassRecord> = RecordManeger::mini_class_rm(&self.config);
        let table_oid = table.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm: RecordManeger<MiniAttributeRecord> = RecordManeger::mini_attribute_rm(&self.config);
        let mut rmgr = RelationManager::new(self.config.clone());
        let mut bm = BufferManager::new(1, self.config.clone());
        let relation = rmgr.get_relation(db_oid, table_oid);
        let mut slot = TupleTableSlot::new(rm.attributes_clone(db_oid, table_oid));
        slot.update_tuple(key_values)?;

        let mut insert = InsertState::new(relation, &slot);
        insert.exec_insert(&mut bm);

        Ok(())
    }
}

impl SelectFromCommnad {
    pub fn new(config: Rc<Config>) -> SelectFromCommnad {
        SelectFromCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, key: &str, value: &str) -> Result<(), String> {
        let db: RecordManeger<MiniDatabaseRecord> = RecordManeger::mini_database_rm(&self.config);
        let db_oid = db.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));

        let table: RecordManeger<MiniClassRecord> = RecordManeger::mini_class_rm(&self.config);
        let table_oid = table.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm: RecordManeger<MiniAttributeRecord> = RecordManeger::mini_attribute_rm(&self.config);
        let mut rmgr = RelationManager::new(self.config.clone());
        let relation = rmgr.get_relation(db_oid, table_oid);
        let mut bm = BufferManager::new(1, self.config.clone());
        let mut scan = ScanState::new(relation, &rm);
        scan.exec_scan(&mut bm);

        Ok(())
    }
}
