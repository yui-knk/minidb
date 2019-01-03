use std::io::{self, Write};
use catalog::catalog::Record;

#[derive(Debug)]
pub struct MiniDatabaseRecord {
    // name of database
    pub name: String,
}

impl Record for MiniDatabaseRecord {
    fn build_from_line(line: String) -> io::Result<Box<MiniDatabaseRecord>> {
        let r = MiniDatabaseRecord { name: line };
        Ok(Box::new(r))
    }

    fn save_to_file(&self, w: &mut Write) -> io::Result<usize> {
        w.write(self.name.as_bytes())
    }
}

impl MiniDatabaseRecord {
    pub fn new(name: String) -> MiniDatabaseRecord {
        MiniDatabaseRecord { name: name }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_build_from_line() {
        let result1 = MiniDatabaseRecord::build_from_line("db1".to_string());
        assert_eq!(result1.is_ok(), true);
        let ok1 = result1.ok().unwrap();
        assert_eq!(ok1.name, "db1".to_string());

        // let result2 = MiniDatabaseRecord::build_from_line("db1".to_string());
        // assert_eq!(result2.is_err(), true);
    }

    #[test]
    fn test_record_save_to_file() {
        let record = MiniDatabaseRecord { name: "db1".to_string() };
        let mut v = Vec::new();
        record.save_to_file(&mut v);
        assert_eq!(v, b"db1");
    }
}
