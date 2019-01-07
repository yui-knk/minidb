use std::io::{self, BufReader, BufRead, BufWriter, Write};
use std::fs::File;
use std::path::{Path};
use config::Config;

pub trait Record {
    fn build_from_line(line: String) -> io::Result<Box<Self>>;
    fn save_to_file(&self, w: &mut Write) -> io::Result<usize>;
}

pub struct RecordManeger<T: Record> {
    name: String,
    pub records: Vec<Box<T>>,
}

impl<T: Record> RecordManeger<T> {
    pub fn new(name: String) -> RecordManeger<T> {
        RecordManeger {
            name: name,
            records: Vec::new(),
        }
    }

    pub fn build_from_config(name: String, config: &Config) -> io::Result<RecordManeger<T>> {
        RecordManeger::build_from_file(name.clone(), config.system_catalog_file_path(name))
    }

    fn build_from_file<P: AsRef<Path>>(name: String, path: P) -> io::Result<RecordManeger<T>> {
        let mut records = Vec::new();
        let f = File::open(path)?;
        let buf = BufReader::new(f);

        for line in buf.lines() {
            let r = T::build_from_line(line.unwrap())?;
            records.push(r);
        }

        Ok(RecordManeger {
            name: name,
            records: records,
        })
    }

    pub fn add_record(&mut self, record: T) {
        self.records.push(Box::new(record))
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        self.save_to_file(config.system_catalog_file_path(&self.name))
    }

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let f = File::create(path)?;
        let mut buf = BufWriter::new(f);

        for record in &self.records {
            record.save_to_file(&mut buf).unwrap();
            buf.write(b"\n").unwrap();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::fs::File;
    use tempfile::NamedTempFile;
    use catalog::mini_database::MiniDatabaseRecord;

    #[test]
    fn test_build_from_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        write!(tmpfile, "foo\nbar\n").unwrap();
        let db: RecordManeger<MiniDatabaseRecord> = RecordManeger::build_from_file("mini_database".to_string(), tmpfile.path()).unwrap();

        assert_eq!(db.records.len(), 2);
        assert_eq!(db.records[0].name, "foo".to_string());
        assert_eq!(db.records[1].name, "bar".to_string());
    }

    #[test]
    fn test_save_to_file() {
        let tmpfile = NamedTempFile::new().unwrap();
        let mut db: RecordManeger<MiniDatabaseRecord> = RecordManeger::new("mini_database".to_string());
        db.add_record(MiniDatabaseRecord::new("baz".to_string()));
        db.add_record(MiniDatabaseRecord::new("fooo".to_string()));
        db.save_to_file(tmpfile.path()).unwrap();

        let mut buf = String::new();
        let mut f = File::open(tmpfile.path()).unwrap();
        f.read_to_string(&mut buf).unwrap();

        assert_eq!(buf, "baz\nfooo\n".to_string());
    }
}
