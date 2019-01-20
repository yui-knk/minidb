use std::rc::Rc;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd};

use errno::{Errno, errno, set_errno};

use buffer_manager::{RelFileNode, BlockNum, InitialBlockNum};
use config::{Config, DEFAULT_BLOCK_SIZE};

pub struct RelationData<'a> {
    // relation physical identifier
    rd_node: RelFileNode,
    rd_smgr: &'a SMgrRelationData,
}

pub struct SMgrRelationData {
    file: File,
    // current insertion target block
    smgr_targblock: BlockNum,
}

pub struct StorageManager {
    config: Rc<Config>,
    cache: HashMap<RelFileNode, SMgrRelationData>
}

impl SMgrRelationData {
    pub fn mdread(&self, buffer: *mut libc::c_void) {
        let s = DEFAULT_BLOCK_SIZE;
        let fd = self.file.as_raw_fd();

        unsafe {
            set_errno(Errno(0));

            let rbyte = libc::read(fd, buffer as *mut libc::c_void, s as usize);

            if rbyte == -1 {
                panic!("Failed to read file. '{}'", errno());
            }

            if (rbyte != 0) && (rbyte != s as isize) {
                panic!(
                    "Failed to read file. Expect to read {} bytes but read only {} bytes",
                    s, rbyte
                );
            }
        }
    }
}

impl StorageManager {
    pub fn new(config: Rc<Config>) -> StorageManager {
        StorageManager {
            config: config,
            cache: HashMap::new(),
        }
    }

    pub fn smgropen(&mut self, rd_node: &RelFileNode) -> &SMgrRelationData {
        let config = &self.config;
        let cache = &mut self.cache;

        cache.entry(rd_node.clone()).or_insert_with(|| {
            // TODO: we should open a file in smgrcreate function (with mdcreate function).
            let path = config.data_file_path(rd_node.db_oid, rd_node.table_oid);
            let f = if path.exists() {
                File::open(&path).unwrap()
            } else {
                // TODO: Should we initalize file explicity?
                // In pg mdopen function create a file only if bootstrap mode.
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(&path)
                    .unwrap()
            };

            SMgrRelationData {
                file: f,
                smgr_targblock: InitialBlockNum,
            }
        })
    }
}
