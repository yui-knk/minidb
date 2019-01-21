use std::path::{Path, PathBuf};

use oid_manager::{Oid, oid_to_string};

pub struct Config {
    root_dir_name: String,
    block_size: u16,
}

pub const DEFAULT_BLOCK_SIZE: u16 = 1024 * 8;
// NBuffers in pg.
pub const N_BUFFERS: usize = 10;

impl Config {
    pub fn new(root_dir_name: String) -> Config {
        Config {
            root_dir_name: root_dir_name,
            block_size: DEFAULT_BLOCK_SIZE,
        }
    }

    // [Catalogs]
    // root directory / "global" / table name /
    // Under the "table name"
    // * "data": table file
    //
    // [User defined tables]
    // root directory / "base" / database oid / table oid /
    // Under the "table name"
    // * "data": table file
    pub fn root_dir_path(&self) -> PathBuf {
        Path::new(&self.root_dir_name).to_path_buf()
    }

    pub fn base_dir_path(&self) -> PathBuf {
        self.root_dir_path().join("base")
    }

    pub fn global_dir_path(&self) -> PathBuf {
        self.root_dir_path().join("global")
    }

    pub fn oid_file_path(&self) -> PathBuf {
        self.global_dir_path().join("oid")
    }

    pub fn system_catalog_dir_path<P: AsRef<Path>>(&self, tablename: P) -> PathBuf {
        self.global_dir_path().join(tablename)
    }

    pub fn system_catalog_file_path<P: AsRef<Path>>(&self, tablename: P) -> PathBuf {
        self.system_catalog_dir_path(tablename).join("data")
    }

    pub fn database_dir_path(&self, db_oid: Oid) -> PathBuf {
        self.base_dir_path().join(oid_to_string(db_oid))
    }

    pub fn table_dir_path(&self, db_oid: Oid, table_oid: Oid) -> PathBuf {
        self.database_dir_path(db_oid).join(oid_to_string(table_oid))
    }

    pub fn data_file_path(&self, db_oid: Oid, table_oid: Oid) -> PathBuf {
        self.table_dir_path(db_oid, table_oid).join("data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.root_dir_path(), PathBuf::from("/mydb"));
    }

    #[test]
    fn test_base_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.base_dir_path(), PathBuf::from("/mydb/base"));
    }

    #[test]
    fn test_global_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.global_dir_path(), PathBuf::from("/mydb/global"));
    }

    #[test]
    fn test_oid_file_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.oid_file_path(), PathBuf::from("/mydb/global/oid"));
    }

    #[test]
    fn test_system_catalog_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.system_catalog_dir_path("mini_database"), PathBuf::from("/mydb/global/mini_database"));
    }

    #[test]
    fn test_system_catalog_file_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.system_catalog_file_path("mini_database"), PathBuf::from("/mydb/global/mini_database/data"));
    }

    #[test]
    fn test_database_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.database_dir_path(10001), PathBuf::from("/mydb/base/10001"));
    }

    #[test]
    fn test_table_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.table_dir_path(10001, 10002), PathBuf::from("/mydb/base/10001/10002"));
    }
    #[test]
    fn test_data_file_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.data_file_path(10001, 10002), PathBuf::from("/mydb/base/10001/10002/data"));
    }
}
