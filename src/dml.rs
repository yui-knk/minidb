use std::rc::Rc;

use ty;
use config::{Config};
use catalog::catalog::RecordManeger;
use catalog::mini_attribute::MiniAttributeRecord;
use tuple::{TupleTableSlot};
use buffer_manager::{RelFileNode, BufferManager};
use node_seqscan::{ScanState};
use node_insert::{InsertState};

pub struct InsertIntoCommnad {
    config: Rc<Config>,
}

pub struct SelectFromCommnad {
    config: Rc<Config>,
}

pub struct KeyValue {
    key: String,
    value: String,
}

pub struct KeyValueBuilder {
    key_values: Vec<KeyValue>,
}

impl KeyValueBuilder {
    pub fn new() -> KeyValueBuilder {
        KeyValueBuilder { key_values: Vec::new() }
    }

    pub fn add_pair(&mut self, key: String, value: String) {
        self.key_values.push(KeyValue {
            key: key,
            value: value,
        })
    }

    pub fn build(self) -> Vec<KeyValue> {
        self.key_values
    }
}

impl InsertIntoCommnad {
    pub fn new(config: Rc<Config>) -> InsertIntoCommnad {
        InsertIntoCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, key_values: Vec<KeyValue>) -> Result<(), String> {
        let rm: RecordManeger<MiniAttributeRecord> = RecordManeger::mini_attribute_rm(&self.config);
        let file_node = RelFileNode {
            table_name: table_name.to_string(),
            dbname: dbname.to_string(),
        };
        let mut bm = BufferManager::new(1, self.config.clone());
        let mut slot = TupleTableSlot::new(rm.attributes_clone(dbname, table_name));
        let attrs = rm.attributes(dbname, table_name);
        if attrs.len() != key_values.len() {
            return Err(format!("Length not match. attrs: {}, key_values: {}", attrs.len(), key_values.len()));
        }

        for ((i, kv), attr) in key_values.iter().enumerate().zip(attrs.iter()) {
            if kv.key != attr.name {
                return Err(format!("Name not match. attrs: {}, key_values: {}", attr.name, kv.key));
            }

            let t = ty::build_ty(&attr.type_name, &kv.value)?;
            slot.set_column(i, t.as_ref());
        }

        let mut insert = InsertState::new(&file_node, &slot);
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
        let rm: RecordManeger<MiniAttributeRecord> = RecordManeger::mini_attribute_rm(&self.config);
        let file_node = RelFileNode {
            table_name: table_name.to_string(),
            dbname: dbname.to_string(),
        };
        let mut bm = BufferManager::new(1, self.config.clone());
        let mut scan = ScanState::new(&file_node, &rm);
        scan.exec_scan(&mut bm);

        Ok(())
    }
}
