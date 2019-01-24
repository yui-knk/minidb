use std::rc::Rc;
use std::collections::HashMap;
use std::io::{Seek, SeekFrom};
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd};
use std::cell::RefCell;

use errno::{Errno, errno, set_errno};

use buffer_manager::{RelFileNode, BlockNum, InvalidBlockNumber};
use config::{Config, DEFAULT_BLOCK_SIZE};
use oid_manager::{Oid, DUMMY_OID};

// `HEAP_DEFAULT_FILLFACTOR` in pg.
// const HEAP_DEFAULT_FILLFACTOR: u8 = 100;

pub struct RelationData {
    // relation physical identifier
    rd_node: RelFileNode,
    pub smgr_rnode: RelFileNode
}

pub struct SMgrRelationData {
    config: Rc<Config>,
    pub smgr_rnode: RelFileNode,
    file: Option<File>,
    // current insertion target block
    pub smgr_targblock: BlockNum,
}

pub struct StorageManager {
    config: Rc<Config>,
    cache: HashMap<RelFileNode, RefCell<SMgrRelationData>>
}

pub struct RelationManager {
    config: Rc<Config>,
    // RelationIdCache in pg.
    cache: HashMap<Oid, RefCell<RelationData>>,
}

impl RelationManager {
    pub fn new(config: Rc<Config>) -> RelationManager {
        RelationManager {
            config: config,
            cache: HashMap::new(),
        }
    }

    pub fn get_relation(&mut self, db_oid: Oid, table_oid: Oid) -> &mut RefCell<RelationData> {
        let cache = &mut self.cache;

        cache.entry(table_oid).or_insert_with(|| {
            let rd_node = RelFileNode {
                table_oid: table_oid,
                db_oid: DUMMY_OID, // TODO
            };
            let smgr_rnode = RelFileNode {
                table_oid: table_oid,
                db_oid: db_oid,
            };

            RefCell::new(
                RelationData {
                    rd_node: rd_node,
                    smgr_rnode: smgr_rnode,
                }
            )
        })
    }
}

impl SMgrRelationData {
    pub fn mdread(&mut self, block_num: BlockNum, buffer: *mut libc::c_void) {
        let s = DEFAULT_BLOCK_SIZE as u32;
        self.mdopen();

        let mut f = self.file.as_ref().unwrap();

        let seekpos = s * block_num;
        if f.seek(SeekFrom::Start(seekpos.into())).unwrap() != seekpos.into() {
            panic!("Failed to seek file. '{}'", seekpos);
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

    pub fn mdwrite(&mut self, block_num: BlockNum, buffer: *const libc::c_void) {
        let s = DEFAULT_BLOCK_SIZE as u32;
        self.mdopen();

        let mut f = self.file.as_ref().unwrap();

        let seekpos = s * block_num;
        if f.seek(SeekFrom::Start(seekpos.into())).unwrap() != seekpos.into() {
            panic!("Failed to seek file. '{}'", seekpos);
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

    pub fn mdextend(&mut self, block_num: BlockNum, buffer: *const libc::c_void) {
        let s = DEFAULT_BLOCK_SIZE as u32;
        self.mdopen();

        let mut f = self.file.as_ref().unwrap();

        // Seek to start of the new page (BlockNum is 0-origin).
        let seekpos = s * block_num;
        if f.seek(SeekFrom::Start(seekpos.into())).unwrap() != seekpos.into() {
            panic!("Failed to seek file. '{}'", seekpos);
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

    pub fn mdnblocks(&mut self) -> BlockNum {
        self.mdopen();
        let mut f = self.file.as_ref().unwrap();
        let len = f.seek(SeekFrom::End(0)).unwrap();
        (len / DEFAULT_BLOCK_SIZE as u64) as BlockNum
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

    pub fn relation_smgropen(&mut self, relation: &RelationData) -> &RefCell<SMgrRelationData> {
        self.smgropen(&relation.smgr_rnode)
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
                    smgr_targblock: InvalidBlockNumber,
                }
            )
        })
    }
}
