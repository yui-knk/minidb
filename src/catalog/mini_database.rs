use std::io::{self, BufReader, BufRead, BufWriter, Write};
use std::fs::File;
use std::path::{Path};
use config::Config;

const NAME: &'static str = "mini_database";

#[derive(Debug)]
struct Record {
    // name of database
    name: String,
}

pub struct MiniDatabase {
    records: Vec<Record>,
}

impl Record {
    pub fn build_from_line(line: String) -> Record {
        Record { name: line }
    }

    pub fn new(name: String) -> Record {
        Record { name: name }
    }
}

impl MiniDatabase {
    pub fn new() -> MiniDatabase {
        MiniDatabase { records: Vec::new() }
    }

    pub fn build_from_config(config: &Config) -> io::Result<MiniDatabase> {
        MiniDatabase::build_from_file(config.system_catalog_file_path(NAME))
    }

    fn build_from_file<P: AsRef<Path>>(path: P) -> io::Result<MiniDatabase> {
        let mut records = Vec::new();
        let f = File::open(path)?;
        let buf = BufReader::new(f);

        for line in buf.lines() {
            records.push(Record::build_from_line(line.unwrap()));
        }

        Ok(MiniDatabase { records: records })
    }

    pub fn name(&self) -> &str {
        &NAME
    }

    pub fn add_record(&mut self, name: String) {
        self.records.push(Record::new(name))
    }

    pub fn save(&self, config: &Config) -> io::Result<()> {
        self.save_to_file(config.system_catalog_file_path(NAME))
    }

    fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let f = File::create(path)?;
        let mut buf = BufWriter::new(f);

        for record in &self.records {
            buf.write(record.name.as_bytes()).unwrap();
            buf.write(b"\n").unwrap();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_build_from_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        write!(tmpfile, "foo\nbar\n").unwrap();
        let db = MiniDatabase::build_from_file(tmpfile.path()).unwrap();

        assert_eq!(db.records.len(), 2);
        assert_eq!(db.records[0].name, "foo".to_string());
        assert_eq!(db.records[1].name, "bar".to_string());
    }

    #[test]
    fn test_save_to_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let mut db = MiniDatabase::new();
        db.add_record("baz".to_string());
        db.add_record("fooo".to_string());
        db.save_to_file(tmpfile.path()).unwrap();

        let mut buf = String::new();
        let mut f = File::open(tmpfile.path()).unwrap();
        f.read_to_string(&mut buf).unwrap();

        assert_eq!(buf, "baz\nfooo\n".to_string());
    }
}
