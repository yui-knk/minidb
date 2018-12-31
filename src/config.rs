use std::path::{Path, PathBuf};

pub struct Config {
    root_dir_name: String,
    block_size: u32,
}

const DEFAULT_BLOCK_SIZE: u32 = 1024 * 8;

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

    pub fn root_base_dir_path(&self) -> PathBuf {
        self.root_dir_path().join("base")
    }

    pub fn root_database_dir_path(&self, dbname: String) -> PathBuf {
        self.root_dir_path().join(dbname)
    }

    pub fn root_table_dir_path(&self, dbname: String, tablename: String) -> PathBuf {
        self.root_database_dir_path(dbname).join(tablename)
    }

    pub fn root_data_file_path(&self, dbname: String, tablename: String) -> PathBuf {
        self.root_table_dir_path(dbname, tablename).join("data")
    }
}
