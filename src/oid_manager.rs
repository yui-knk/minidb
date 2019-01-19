use std::io::{BufReader, BufRead, Write};
use std::rc::Rc;
use std::fs::File;

use config::{Config};


// In postgres_ext.h
// typedef unsigned int Oid;
pub type Oid = u32;

// Oid less than InitialOid is for system usage,
// for example databases, tables, attributes...
pub const InitialOid: Oid = 10000;

pub fn create_oid_file(config: &Config) -> std::io::Result<()> {
    let mut f = File::create(config.oid_file_path())?;
    f.write(format!("{}", InitialOid).as_bytes()).map(|_| ())
}

pub struct OidManager {
    config: Rc<Config>,
    current_oid: Oid,
}

impl Drop for OidManager {
    fn drop(&mut self) {
        let path = self.config.oid_file_path();
        let mut f = File::create(path).unwrap();
        f.write(format!("{}", self.current_oid).as_bytes()).unwrap();
    }
}

impl OidManager {
    pub fn new(config: Rc<Config>) -> OidManager {
        let path = config.oid_file_path();
        let f = File::open(path).unwrap();
        let buf = BufReader::new(f);
        let mut lines = buf.lines();
        let oid = lines.nth(0).unwrap().unwrap().parse::<Oid>().expect("Oid file should contain integer.");

        OidManager {
            config: config,
            current_oid: oid,
        }
    }

    pub fn get_new_oid(&mut self) -> Oid {
        let result = self.current_oid;
        self.current_oid = self.current_oid + 1;
        result
    }
}
