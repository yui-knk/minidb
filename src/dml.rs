use std::rc::Rc;
use std::sync::RwLock;

use config::{Config};
use tuple::{TupleTableSlot, KeyValue};
use buffer_manager::{BufferManager};
use storage_manager::{RelationManager};
use executor::node_agg::{CountState};
use executor::node_delete::{DeleteState};
use executor::node_insert::{InsertState};
use executor::node_seqscan::{ScanState};
use executor::node_sort::{SortState};
use executor::plan_node::PlanNode;
use catalog::catalog_manager::CatalogManager;
use ast::Expr;

pub struct InsertIntoCommand {
    config: Rc<Config>,
}

pub struct SelectFromCommand {
    config: Rc<Config>,
}

pub struct CountCommand {
    config: Rc<Config>,
}

pub struct DeleteCommand {
    config: Rc<Config>,
}

impl InsertIntoCommand {
    pub fn new(config: Rc<Config>) -> InsertIntoCommand {
        InsertIntoCommand {
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
        let bm = RwLock::new(BufferManager::new(1, self.config.clone()));
        let relation = rmgr.get_relation(db_oid, table_oid);
        let mut slot = TupleTableSlot::new(rm.attributes_clone(db_oid, table_oid));
        slot.update_tuple(key_values)?;

        let mut insert = InsertState::new(relation, &slot, &bm);
        insert.exec();

        Ok(())
    }
}

impl SelectFromCommand {
    pub fn new(config: Rc<Config>) -> SelectFromCommand {
        SelectFromCommand {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, cmgr: &CatalogManager, qual: &Option<Box<Expr>>, sort: &Option<String>) -> Result<(), String> {
        let db_oid = cmgr.database_rm.find_mini_database_oid(dbname)
                       .expect(&format!("{} database should be defined.", dbname));
        let table_oid = cmgr.class_rm.find_mini_class_oid(db_oid, table_name)
                             .expect(&format!("{} table should be defined under the {} database. ", table_name, dbname));
        let rm = &cmgr.attribute_rm;
        let mut rmgr = RelationManager::new(self.config.clone());
        let relation = rmgr.get_relation(db_oid, table_oid);
        let bm = RwLock::new(BufferManager::new(1, self.config.clone()));

        match sort {
            Some(col_name) => {
                let mut scan = ScanState::new(relation, &rm, &bm, qual);
                let mut sort_state = SortState::new(&mut scan, col_name.clone());

                loop {
                    let opt = sort_state.exec();

                    match opt {
                        Some(slot) => {
                            for i in 0..(slot.attrs_count()) {
                                let ty = slot.get_column(i);
                                print!("{:?} ", ty.as_string());
                            }
                            print!("\n");
                        },
                        None => break
                    }
                }
            },
            None => {
                let mut scan = ScanState::new(relation, &rm, &bm, qual);

                loop {
                    let opt = scan.exec();

                    match opt {
                        Some(slot) => {
                            for i in 0..(slot.attrs_count()) {
                                let ty = slot.get_column(i);
                                print!("{:?} ", ty.as_string());
                            }
                            print!("\n");
                        },
                        None => break
                    }
                }
            }
        }

        Ok(())
    }
}

impl CountCommand {
    pub fn new(config: Rc<Config>) -> CountCommand {
        CountCommand {
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
        let mut count = CountState::new(&mut scan);

        count.exec();
        println!("Count: {}", count.result);

        Ok(())
    }
}

impl DeleteCommand {
    pub fn new(config: Rc<Config>) -> DeleteCommand {
        DeleteCommand {
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
        let mut delete = DeleteState::new(relation, &mut scan, &bm);

        delete.exec();
        println!("Deleted records: {}", delete.count);

        Ok(())
    }
}
