use ty;
use std::fs::File;
use page::{Page};
use config::Config;
use catalog::catalog::RecordManeger;
use catalog::mini_attribute::MiniAttributeRecord;
use config::DEFAULT_BLOCK_SIZE;

pub struct InsertIntoCommnad<'a> {
    config: &'a Config,
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

impl<'a> InsertIntoCommnad<'a> {
    pub fn new(config: &'a Config) -> InsertIntoCommnad<'a> {
        InsertIntoCommnad {
            config: config,
        }
    }

    pub fn execute(&self, dbname: &str, table_name: &str, key_values: Vec<KeyValue>) -> Result<(), String> {
        let rm: RecordManeger<MiniAttributeRecord> = RecordManeger::build_from_config("mini_attribute".to_string(), self.config).unwrap();
        let attrs = rm.attributes(dbname, table_name);
        let path = self.config.data_file_path(dbname, table_name);
        let mut page = if path.exists() {
            Page::load(&path)
        } else {
            Page::new(DEFAULT_BLOCK_SIZE)
        };

        if attrs.len() != key_values.len() {
            return Err(format!("Length not match. attrs: {}, key_values: {}", attrs.len(), key_values.len()));
        }

        for (kv, attr) in key_values.iter().zip(attrs.iter()) {
            if kv.key != attr.name {
                return Err(format!("Name not match. attrs: {}, key_values: {}", attr.name, kv.key));;
            }

            let t = ty::build_ty(&attr.type_name, &kv.value)?;
            let mut buf = Vec::new();
            t.write_bytes(&mut buf).unwrap();
            page.add_entry(&buf);
        }

        let f = File::create(path).unwrap();
        page.write_bytes(f);

        Ok(())
    }
}
