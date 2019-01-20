use std::rc::Rc;
use std::collections::HashMap;
use std::io::{Seek, SeekFrom};
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd};
use std::cell::RefCell;

use errno::{Errno, errno, set_errno};

use buffer_manager::{RelFileNode, BlockNum, InitialBlockNum};
use config::{Config, DEFAULT_BLOCK_SIZE};

pub struct RelationData<'a> {
    // relation physical identifier
    rd_node: RelFileNode,
    rd_smgr: &'a SMgrRelationData,
}

pub struct SMgrRelationData {
    config: Rc<Config>,
    smgr_rnode: RelFileNode,
    file: Option<File>,
    // current insertion target block
    smgr_targblock: BlockNum,
}

pub struct StorageManager {
    config: Rc<Config>,
    cache: HashMap<RelFileNode, RefCell<SMgrRelationData>>
}

impl SMgrRelationData {
    pub fn mdread(&mut self, buffer: *mut libc::c_void) {
        let s = DEFAULT_BLOCK_SIZE;
        self.mdopen();

        let mut f = self.file.as_ref().unwrap();

        let offset = 0;
        if f.seek(SeekFrom::Start(offset)).unwrap() != offset {
            panic!("Failed to seek file. '{}'", offset);
        }

        let fd = f.as_raw_fd();

        unsafe {
            set_errno(Errno(0));

            let rbyte = libc::read(fd, buffer, s as usize);

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

    pub fn mdwrite(&mut self, buffer: *const libc::c_void) {
        let s = DEFAULT_BLOCK_SIZE;
        self.mdopen();
        let mut f = self.file.as_ref().unwrap();

        let offset = 0;
        if f.seek(SeekFrom::Start(offset)).unwrap() != offset {
            panic!("Failed to seek file. '{}'", offset);
        }

        let fd = f.as_raw_fd();

        unsafe {
            set_errno(Errno(0));

            let wbyte = libc::write(fd, buffer, s as usize);

            if wbyte == -1 {
                panic!("Failed to write file. '{}'", errno());
            }

            if wbyte != s as isize {
                panic!(
                    "failed to write file. Expect to write {} bytes but write only {} bytes",
                    s, wbyte
                );
            }
        }
    }

    fn mdopen(&mut self) {
        if self.file.is_some() {
            return
        }

        let path = self.config.data_file_path(self.smgr_rnode.db_oid, self.smgr_rnode.table_oid);
        // TODO: Should we initalize file explicity?
        // In pg mdopen function create a file only if bootstrap mode.
        let f = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(&path)
                    .unwrap();

        self.file = Some(f);
    }
}

impl StorageManager {
    pub fn new(config: Rc<Config>) -> StorageManager {
        StorageManager {
            config: config,
            cache: HashMap::new(),
        }
    }

    pub fn smgropen(&mut self, rd_node: &RelFileNode) -> &RefCell<SMgrRelationData> {
        let config = &self.config;
        let cache = &mut self.cache;

        cache.entry(rd_node.clone()).or_insert_with(|| {
            RefCell::new(
                SMgrRelationData {
                    config: config.clone(),
                    smgr_rnode: rd_node.clone(),
                    file: None,
                    smgr_targblock: InitialBlockNum,
                }
            )
        })
    }
}
