// This is for columns.

use std::io::{self, Error, ErrorKind, Write};

use config::Config;
use catalog::catalog::{Record, RecordManeger};

#[derive(Debug, Clone, PartialEq)]
pub enum TypeLabel {
    // Signed 4 bytes integer
    Integer,
}

fn ty_to_u32(ty: &TypeLabel) -> u32 {
    match ty {
        Integer => 1,
    }
}

fn u32_to_ty(i: u32) -> TypeLabel {
    match i {
        1 => Integer,
        _ => panic!("Unknown type {}", i)
    }
}

fn ty_byte_len(ty: &TypeLabel) -> u16 {
    match ty {
        Integer => 4,
    }
}

use self::TypeLabel::*;

#[derive(Debug, Clone)]
pub struct MiniAttributeRecord {
    // name of attribute
    pub name: String,
    // name of db this attribute belongs to
    pub dbname: String,
    // name of class this attribute belongs to
    pub class_name: String,
    pub ty: TypeLabel,
    // Byte length of value
    pub len: usize,
}

impl Record for MiniAttributeRecord {
    fn build_from_line(line: String) -> io::Result<Box<MiniAttributeRecord>> {
        let c: Vec<&str> = line.split(",").collect();

        if c.len() != 5 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Line ({}) is invalid.", line)
            ));
        }

        let r = MiniAttributeRecord {
            name: c[0].to_string(),
            dbname: c[1].to_string(),
            class_name: c[2].to_string(),
            ty: u32_to_ty(c[3].to_string().parse::<u32>().unwrap()),
            len: c[4].to_string().parse::<usize>().unwrap(),
        };
        Ok(Box::new(r))
    }

    fn save_to_file(&self, w: &mut Write) -> io::Result<usize> {
        w.write(format!(
            "{},{},{},{},{}",
            self.name,
            self.dbname,
            self.class_name,
            ty_to_u32(&self.ty),
            self.len).as_bytes())
    }
}

impl MiniAttributeRecord {
    pub fn new(name: String, dbname: String, class_name: String, ty: TypeLabel, len: usize) -> MiniAttributeRecord {
        MiniAttributeRecord {
            name: name,
            dbname: dbname,
            class_name: class_name,
            ty: ty,
            len: len
        }
    }

    fn byte_len(&self) -> u16 {
        ty_byte_len(&self.ty)
    }
}

// TODO: Define `Vec<&MiniAttributeRecord>` as struct.
impl RecordManeger<MiniAttributeRecord> {
    pub fn mini_attribute_rm(config: &Config) -> RecordManeger<MiniAttributeRecord> {
        RecordManeger::build_from_config("mini_attribute".to_string(), config).unwrap()
    }

    pub fn attributes(&self, dbname: &str, table_name: &str) -> Vec<&MiniAttributeRecord> {
        self.records
            .iter()
            .filter(|e| e.dbname == dbname && e.class_name == table_name)
            .map(|e| e.as_ref())
            .collect()
    }

    pub fn attributes_clone(&self, dbname: &str, table_name: &str) -> Vec<MiniAttributeRecord> {
        self.records
            .iter()
            .filter(|e| e.dbname == dbname && e.class_name == table_name)
            .map(|e| *e.clone())
            .collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_build_from_line() {
        let result1 = MiniAttributeRecord::build_from_line("id,db2,table1,1,4".to_string());
        assert_eq!(result1.is_ok(), true);
        let ok1 = result1.ok().unwrap();
        assert_eq!(ok1.name, "id".to_string());
        assert_eq!(ok1.dbname, "db2".to_string());
        assert_eq!(ok1.class_name, "table1".to_string());
        assert_eq!(ok1.ty, Integer);
        assert_eq!(ok1.len, 4);

        let result2 = MiniAttributeRecord::build_from_line("table1".to_string());
        assert_eq!(result2.is_err(), true);
    }

    #[test]
    fn test_record_save_to_file() {
        let record = MiniAttributeRecord {
            name: "id".to_string(),
            dbname: "db2".to_string(),
            class_name: "table1".to_string(),
            ty: Integer,
            len: 4,
        };
        let mut v = Vec::new();
        record.save_to_file(&mut v);
        assert_eq!(v, b"id,db2,table1,1,4");
    }
}
