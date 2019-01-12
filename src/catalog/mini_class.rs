// This is for most everything that has columns or is otherwise
// similar to a table. For example "table".

use std::io::{self, Error, ErrorKind, Write};

use config::Config;
use catalog::catalog::{Record, RecordManeger};

#[derive(Debug)]
pub struct MiniClassRecord {
    // name of table
    name: String,
    dbname: String
}

impl Record for MiniClassRecord {
    fn build_from_line(line: String) -> io::Result<Box<MiniClassRecord>> {
        let c: Vec<&str> = line.split(",").collect();

        if c.len() != 2 {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Line ({}) is invalid.", line)
            ));
        }

        let r = MiniClassRecord {
            name: c[0].to_string(),
            dbname: c[1].to_string(),
        };
        Ok(Box::new(r))
    }

    fn save_to_file(&self, w: &mut Write) -> io::Result<usize> {
        w.write(format!("{},{}", self.name, self.dbname).as_bytes())
    }
}

impl MiniClassRecord {
    pub fn new(name: String, dbname: String) -> MiniClassRecord {
        MiniClassRecord { name: name, dbname: dbname }
    }
}

impl RecordManeger<MiniClassRecord> {
    pub fn mini_class_rm(config: &Config) -> RecordManeger<MiniClassRecord> {
        RecordManeger::build_from_config("mini_class".to_string(), config).unwrap()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_build_from_line() {
        let result1 = MiniClassRecord::build_from_line("table1,db2".to_string());
        assert_eq!(result1.is_ok(), true);
        let ok1 = result1.ok().unwrap();
        assert_eq!(ok1.name, "table1".to_string());
        assert_eq!(ok1.dbname, "db2".to_string());

        let result2 = MiniClassRecord::build_from_line("table1".to_string());
        assert_eq!(result2.is_err(), true);
    }

    #[test]
    fn test_record_save_to_file() {
        let record = MiniClassRecord { name: "table1".to_string(), dbname: "db2".to_string() };
        let mut v = Vec::new();
        record.save_to_file(&mut v);
        assert_eq!(v, b"table1,db2");
    }
}
