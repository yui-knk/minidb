// This is for columns.

use std::borrow::Borrow;
use std::io::{self, Error, ErrorKind, Write};
use catalog::catalog::{Record, RecordManeger};

#[derive(Debug)]
pub struct MiniAttributeRecord {
    // name of attribute
    pub name: String,
    // name of db this attribute belongs to
    pub dbname: String,
    // name of class this attribute belongs to
    pub class_name: String,
    pub type_name: String,
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
            type_name: c[3].to_string(),
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
            self.type_name,
            self.len).as_bytes())
    }
}

impl MiniAttributeRecord {
    pub fn new(name: String, dbname: String, class_name: String, type_name: String, len: usize) -> MiniAttributeRecord {
        MiniAttributeRecord {
            name: name,
            dbname: dbname,
            class_name: class_name,
            type_name: type_name,
            len: len
        }
    }
}

impl RecordManeger<MiniAttributeRecord> {
    pub fn attributes(&self, dbname: &str, table_name: &str) -> Vec<&MiniAttributeRecord> {
        self.records
            .iter()
            .filter(|e| e.dbname == dbname && e.class_name == table_name)
            .map(|e| e.borrow())
            .collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_build_from_line() {
        let result1 = MiniAttributeRecord::build_from_line("id,db2,table1,integer,4".to_string());
        assert_eq!(result1.is_ok(), true);
        let ok1 = result1.ok().unwrap();
        assert_eq!(ok1.name, "id".to_string());
        assert_eq!(ok1.dbname, "db2".to_string());
        assert_eq!(ok1.class_name, "table1".to_string());
        assert_eq!(ok1.type_name, "integer".to_string());
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
            type_name: "integer".to_string(),
            len: 4,
        };
        let mut v = Vec::new();
        record.save_to_file(&mut v);
        assert_eq!(v, b"id,db2,table1,integer,4");
    }
}
