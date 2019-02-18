use std::rc::Rc;
use std::sync::RwLock;

use config::{Config};
use tuple::{TupleTableSlot, KeyValue};
use buffer_manager::{BufferManager};
use storage_manager::{RelationManager};
use node_seqscan::{ScanState};
use node_insert::{InsertState};
use node_agg::{CountState};
use node_delete::{DeleteState};
use catalog::catalog_manager::CatalogManager;
use ast::Expr;

pub struct InsertIntoCommnad {
    config: Rc<Config>,
}

pub struct SelectFromCommnad {
    config: Rc<Config>,
}

pub struct CountCommnad {
    config: Rc<Config>,
}

pub struct DeleteCommnad {
    config: Rc<Config>,
}

impl InsertIntoCommnad {
    pub fn new(config: Rc<Config>) -> InsertIntoCommnad {
        InsertIntoCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, key_values: Vec<KeyValue>, cmgr: &CatalogManager) -> Result<(), String> {
        let db_oid = cmgr.database_rm.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));
        let table_oid = cmgr.class_rm.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm = &cmgr.attribute_rm;
        let mut rmgr = RelationManager::new(self.config.clone());
        let mut bm = RwLock::new(BufferManager::new(1, self.config.clone()));
        let relation = rmgr.get_relation(db_oid, table_oid);
        let mut slot = TupleTableSlot::new(rm.attributes_clone(db_oid, table_oid));
        slot.update_tuple(key_values)?;

        let mut insert = InsertState::new(relation, &slot, &bm);
        insert.exec_insert();

        Ok(())
    }
}

impl SelectFromCommnad {
    pub fn new(config: Rc<Config>) -> SelectFromCommnad {
        SelectFromCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, cmgr: &CatalogManager, qual: &Option<Box<Expr>>) -> Result<(), String> {
        let db_oid = cmgr.database_rm.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));
        let table_oid = cmgr.class_rm.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm = &cmgr.attribute_rm;
        let mut rmgr = RelationManager::new(self.config.clone());
        let relation = rmgr.get_relation(db_oid, table_oid);
        let bm = RwLock::new(BufferManager::new(1, self.config.clone()));
        let mut scan = ScanState::new(relation, &rm, &bm, qual);

        loop {
            let opt = scan.exec_scan();

            match opt {
                Some(slot) => {
                    for i in 0..(slot.attrs_count()) {
                        let ty = slot.get_column(i);
                        println!("{:?}", ty.as_string());
                    }
                },
                None => break
            }
        }

        Ok(())
    }
}

impl CountCommnad {
    pub fn new(config: Rc<Config>) -> CountCommnad {
        CountCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, cmgr: &CatalogManager, qual: &Option<Box<Expr>>) -> Result<(), String> {
        let db_oid = cmgr.database_rm.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));
        let table_oid = cmgr.class_rm.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm = &cmgr.attribute_rm;
        let mut rmgr = RelationManager::new(self.config.clone());
        let relation = rmgr.get_relation(db_oid, table_oid);
        let bm = RwLock::new(BufferManager::new(1, self.config.clone()));
        let scan = ScanState::new(relation, &rm, &bm, qual);
        let mut count = CountState::new(scan);

        count.exec_agg();
        println!("Count: {}", count.result);

        Ok(())
    }
}

impl DeleteCommnad {
    pub fn new(config: Rc<Config>) -> DeleteCommnad {
        DeleteCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, cmgr: &CatalogManager, qual: &Option<Box<Expr>>) -> Result<(), String> {
        let db_oid = cmgr.database_rm.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));
        let table_oid = cmgr.class_rm.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm = &cmgr.attribute_rm;
        let mut rmgr = RelationManager::new(self.config.clone());
        let relation = rmgr.get_relation(db_oid, table_oid);
        let bm = RwLock::new(BufferManager::new(1, self.config.clone()));
        let scan = ScanState::new(relation, &rm, &bm, qual);
        let mut delete = DeleteState::new(relation, scan, &bm);

        delete.exec_delete();
        println!("Deleted records: {}", delete.count);

        Ok(())
    }
}
