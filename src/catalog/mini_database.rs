use std::io::{self, Error, ErrorKind, Write};

use config::Config;
use catalog::catalog::{Record, RecordManeger};
use oid_manager::Oid;

#[derive(Debug)]
pub struct MiniDatabaseRecord {
    // oid of database
    pub oid: Oid,
    // name of database
    pub name: String,
}

impl Record for MiniDatabaseRecord {
    fn build_from_line(line: String) -> io::Result<Box<MiniDatabaseRecord>> {
        let c: Vec<&str> = line.split(",").collect();

        if c.len() != 2 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Line ({}) is invalid.", line)
            ));
        }

        let r = MiniDatabaseRecord {
            oid: c[0].to_string().parse::<u32>().unwrap(),
            name: c[1].to_string(),
        };
        Ok(Box::new(r))
    }

    fn save_to_file(&self, w: &mut Write) -> io::Result<usize> {
        w.write(format!(
            "{},{}",
            self.oid,
            self.name
        ).as_bytes())
    }
}

impl MiniDatabaseRecord {
    pub fn new(oid: Oid, name: String) -> MiniDatabaseRecord {
        MiniDatabaseRecord {
            oid: oid,
            name: name
        }
    }
}

impl RecordManeger<MiniDatabaseRecord> {
    pub fn mini_database_rm(config: &Config) -> RecordManeger<MiniDatabaseRecord> {
        RecordManeger::build_from_config("mini_database".to_string(), config).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_build_from_line() {
        let result1 = MiniDatabaseRecord::build_from_line("10001,db1".to_string());
        assert_eq!(result1.is_ok(), true);
        let ok1 = result1.ok().unwrap();
        assert_eq!(ok1.oid, 10001);
        assert_eq!(ok1.name, "db1".to_string());

        // let result2 = MiniDatabaseRecord::build_from_line("db1".to_string());
        // assert_eq!(result2.is_err(), true);
    }

    #[test]
    fn test_record_save_to_file() {
        let record = MiniDatabaseRecord {
            oid: 10002,
            name: "db1".to_string()
        };
        let mut v = Vec::new();
        record.save_to_file(&mut v);
        assert_eq!(v, b"10002,db1");
    }
}
