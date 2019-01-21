use std::rc::Rc;
use std::collections::HashMap;
use std::fs::File;

use page::{Page};
use config::{Config, N_BUFFERS, DEFAULT_BLOCK_SIZE};
use oid_manager::Oid;
use storage_manager::StorageManager;

// Buffer identifiers
// Zero is invalid, positive is the index of a shared buffer (1..NBuffers),
// negative is the index of a local buffer (-1 .. -NLocBuffer).
#[derive(Debug, Clone, Copy)]
pub enum Buffer {
    InvalidBuffer,
    Buffer(usize)
}

// Block number of a data file (start with 0)
pub type BlockNum = u32;
pub const InitialBlockNum: BlockNum = 0;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct RelFileNode {
    pub table_oid: Oid,
    pub db_oid: Oid,
}

// `typedef struct buftag {} BufferTag` in pg.
#[derive(Hash, Eq, PartialEq, Debug)]
struct BufferTag {
    rnode: RelFileNode,
    block_num: BlockNum,
}

struct BufferDesc {
    tag: BufferTag,
    buf_id: Buffer, // buffer's index number (from 0)
    locked: bool,
    dirty: bool,
    valid: bool,
}

pub struct BufferManager {
    config: Rc<Config>,
    smgr: StorageManager,
    buffer_descriptors: Vec<BufferDesc>,
    pages: Vec<Page>,
    // Hash from BufferTag to index of descriptor and page
    buffer_hash: HashMap<BufferTag, Buffer>,
}

impl Drop for BufferManager {
    fn drop(&mut self) {
        let len = self.buffer_descriptors.len();

        for i in 0..len {
            let page = &self.pages[i];
            let descriptor = &self.buffer_descriptors[i];
            let rnode = &descriptor.tag.rnode;

            let mut relation_data = self.smgr.smgropen(&rnode);
            relation_data.borrow_mut().mdwrite(page.header_pointer());
        }
    }
}

impl BufferManager {
    pub fn new(size: usize, config: Rc<Config>, smgr: StorageManager) -> BufferManager {
        BufferManager {
            config: config,
            smgr: smgr,
            buffer_descriptors: Vec::with_capacity(N_BUFFERS),
            pages: Vec::with_capacity(N_BUFFERS),
            buffer_hash: HashMap::new(),
        }
    }

    // `ReadBuffer` function in pg.
    // This should recieve Relation instead of RelFileNode because we should
    // determine which block should be loaded, but the block info is stored in
    // Relation (SMgrRelationData). Maby new method RelationGetBufferForTuple (hio.c)
    // is needed.
    pub fn read_buffer(&mut self, file_node: RelFileNode, block_num: BlockNum) -> Buffer {
        let page = Page::new(DEFAULT_BLOCK_SIZE);
        let mut relation_data = self.smgr.smgropen(&file_node);
        relation_data.borrow_mut().mdread(page.header_pointer());
        // TODO: Check length
        let buffer = Buffer::Buffer(self.pages.len());
        let tag = BufferTag {
            rnode: file_node,
            block_num: block_num,
        };
        let descriptor = BufferDesc {
            tag: tag,
            buf_id: buffer,
            locked: false,
            dirty: false,
            valid: true,
        };
        self.pages.push(page);
        self.buffer_descriptors.push(descriptor);

        buffer
    }

    pub fn get_page(&self, buffer_id: Buffer) -> &Page {
        match buffer_id {
            Buffer::Buffer(buf) => &self.pages[buf],
            Buffer::InvalidBuffer => panic!("InvalidBuffer")
        }
    }

    pub fn get_page_mut(&mut self, buffer_id: Buffer) -> &mut Page {
        match buffer_id {
            Buffer::Buffer(buf) => &mut self.pages[buf],
            Buffer::InvalidBuffer => panic!("InvalidBuffer")
        }
    }
    // 
    // pub fn read_buffer_extended(&mut self, block_num: BlockNum) -> Buffer {
    // }
}

