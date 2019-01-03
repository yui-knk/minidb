// This is for most everything that has columns or is otherwise
// similar to a table. For example "table".

use catalog::catalog::Record;

#[derive(Debug)]
pub struct MiniClassRecord {
    // name of table
    name: String,
    dbname: String
}

impl Record for MiniClassRecord {
    fn build_from_line(line: String) -> MiniClassRecord {
        // let c = line.split(",").collect();

        MiniClassRecord {
            name: "".to_string(),
            dbname: line,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        self.name.as_bytes()
    }
}

impl MiniClassRecord {
    pub fn new(name: String, dbname: String) -> MiniClassRecord {
        MiniClassRecord { name: name, dbname: dbname }
    }
}
