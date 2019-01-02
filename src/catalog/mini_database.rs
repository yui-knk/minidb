use catalog::catalog::Record;

#[derive(Debug)]
pub struct MiniDatabaseRecord {
    // name of database
    name: String,
}

impl Record for MiniDatabaseRecord {
    fn build_from_line(line: String) -> MiniDatabaseRecord {
        MiniDatabaseRecord { name: line }
    }

    fn as_bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }
}

impl MiniDatabaseRecord {
    pub fn new(name: String) -> MiniDatabaseRecord {
        MiniDatabaseRecord { name: name }
    }
}
