use std::rc::Rc;

use ty;
use config::{Config};
use catalog::catalog::RecordManeger;
use catalog::mini_attribute::MiniAttributeRecord;
use tuple::{TupleTableSlot};
use buffer_manager::{RelFileNode, BufferManager};

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
        let attrs = rm.attributes(dbname, table_name);
        let mut slot = TupleTableSlot::new(rm.attributes_clone(dbname, table_name));

        let mut bm = BufferManager::new(1, self.config.clone());
        let file_node = RelFileNode {
            table_name: table_name.to_string(),
            dbname: dbname.to_string(),
        };
        let buf = bm.read_buffer(file_node, 0);
        let page = bm.get_page_mut(buf);

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

        page.add_tuple_slot_entry(&slot).unwrap();

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
        let attrs = rm.attributes_clone(dbname, table_name);
        let attrs_len = attrs.iter().fold(0, |acc, attr| acc + attr.len) as u32;
        let mut slot = TupleTableSlot::new(attrs);

        let mut bm = BufferManager::new(1, self.config.clone());
        let file_node = RelFileNode {
            table_name: table_name.to_string(),
            dbname: dbname.to_string(),
        };
        let buf = bm.read_buffer(file_node, 0);
        let page = bm.get_page(buf);

        for i in 0..(page.entry_count()) {
            slot.load_data(page.get_entry_pointer(i).unwrap(), attrs_len);
            println!("attrs_count: {:?}", slot.attrs_count());

            for j in 0..(slot.attrs_count()) {
                let ty = slot.get_column(j);
                println!("{:?}", ty.as_string());
            }
        }

        Ok(())
    }
}
