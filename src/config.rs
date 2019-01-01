use std::path::{Path, PathBuf};

pub struct Config {
    root_dir_name: String,
    block_size: u16,
}

pub const DEFAULT_BLOCK_SIZE: u16 = 1024 * 8;

impl Config {
    pub fn new(root_dir_name: String) -> Config {
        Config {
            root_dir_name: root_dir_name,
            block_size: DEFAULT_BLOCK_SIZE,
        }
    }

    // root directory / "base" / database name / table name /
    //
    // Under the "table name"
    // * "data": table file
    pub fn root_dir_path(&self) -> PathBuf {
        Path::new(&self.root_dir_name).to_path_buf()
    }

    pub fn base_dir_path(&self) -> PathBuf {
        self.root_dir_path().join("base")
    }

    pub fn database_dir_path(&self, dbname: String) -> PathBuf {
        self.base_dir_path().join(dbname)
    }

    pub fn table_dir_path(&self, dbname: String, tablename: String) -> PathBuf {
        self.database_dir_path(dbname).join(tablename)
    }

    pub fn data_file_path(&self, dbname: String, tablename: String) -> PathBuf {
        self.table_dir_path(dbname, tablename).join("data")
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
    fn test_database_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.database_dir_path("db1".to_string()), PathBuf::from("/mydb/base/db1"));
    }

    #[test]
    fn test_table_dir_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.table_dir_path("db1".to_string(), "table1".to_string()), PathBuf::from("/mydb/base/db1/table1"));
    }
    #[test]
    fn test_data_file_path() {
        let config = Config::new("/mydb".to_string());

        assert_eq!(config.data_file_path("db1".to_string(), "table1".to_string()), PathBuf::from("/mydb/base/db1/table1/data"));
    }
}
