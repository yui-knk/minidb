// This is for most everything that has columns or is otherwise
// similar to a table. For example "table".

use std::io::{self, Error, ErrorKind, Write};

use config::Config;
use catalog::catalog::{Record, RecordManeger};
use oid_manager::Oid;

#[derive(Debug)]
pub struct MiniClassRecord {
    // oid of table
    oid: Oid,
    // name of table
    name: String,
    db_oid: Oid
}

impl Record for MiniClassRecord {
    fn build_from_line(line: String) -> io::Result<Box<MiniClassRecord>> {
        let c: Vec<&str> = line.split(",").collect();

        if c.len() != 3 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Line ({}) is invalid.", line)
            ));
        }

        let r = MiniClassRecord {
            oid: c[0].to_string().parse::<u32>().unwrap(),
            name: c[1].to_string(),
            db_oid: c[2].to_string().parse::<u32>().unwrap(),
        };
        Ok(Box::new(r))
    }

    fn save_to_file(&self, w: &mut Write) -> io::Result<usize> {
        w.write(format!(
            "{},{},{}",
            self.oid,
            self.name,
            self.db_oid
        ).as_bytes())
    }
}

impl MiniClassRecord {
    pub fn new(oid: Oid, name: String, db_oid: Oid) -> MiniClassRecord {
        MiniClassRecord {
            oid: oid,
            name: name,
            db_oid: db_oid
        }
    }
}

impl RecordManeger<MiniClassRecord> {
    pub fn mini_class_rm(config: &Config) -> RecordManeger<MiniClassRecord> {
        RecordManeger::build_from_config("mini_class".to_string(), config).unwrap()
    }

    pub fn find_mini_class_oid(&self, db_oid: Oid, name: &str) -> Option<Oid> {
        self.find_mini_class(db_oid, name).map(|c| c.oid)
    }

    fn find_mini_class(&self, db_oid: Oid, name: &str) -> Option<&MiniClassRecord> {
        self.records.iter().find(|e| e.name == name && e.db_oid == db_oid).map(|b| b.as_ref())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_build_from_line() {
        let result1 = MiniClassRecord::build_from_line("10004,table1,10010".to_string());
        assert_eq!(result1.is_ok(), true);
        let ok1 = result1.ok().unwrap();
        assert_eq!(ok1.oid, 10004);
        assert_eq!(ok1.name, "table1".to_string());
        assert_eq!(ok1.db_oid, 10010);

        let result2 = MiniClassRecord::build_from_line("table1".to_string());
        assert_eq!(result2.is_err(), true);
    }

    #[test]
    fn test_record_save_to_file() {
        let record = MiniClassRecord {
            oid: 10005,
            name: "table1".to_string(),
            db_oid: 10006
        };
        let mut v = Vec::new();
        record.save_to_file(&mut v);
        assert_eq!(v, b"10005,table1,10006");
    }
}
