#![allow(non_upper_case_globals)]

use std::rc::Rc;
use std::collections::HashMap;

use page::{Page, MAX_HEAP_TUPLE_SIZE};
use tuple::{TupleTableSlot, HeapTupleData, ItemPointerData};
use config::{Config, N_BUFFERS, DEFAULT_BLOCK_SIZE};
use oid_manager::Oid;
use storage_manager::{StorageManager, RelationData};

// Buffer identifiers
// Zero is invalid, positive is the index of a shared buffer (1..NBuffers),
// negative is the index of a local buffer (-1 .. -NLocBuffer).
#[derive(Debug, Clone, Copy)]
pub enum Buffer {
    InvalidBuffer,
    Buffer(usize)
}

fn unwrap_buffer_id(buffer_id: Buffer) -> usize {
    match buffer_id {
        Buffer::Buffer(buf) => buf,
        Buffer::InvalidBuffer => panic!("InvalidBuffer")
    }
}

// block.h in pg.
// Block number of a data file (start with 0)
pub type BlockNumber = u32;
pub const InvalidBlockNumber: BlockNumber = 0xFFFFFFFF;
// BlockIdData in pg (which is pair of uint16).
pub type BlockIdData = BlockNumber;


#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct RelFileNode {
    pub table_oid: Oid,
    pub db_oid: Oid,
}

// `typedef struct buftag {} BufferTag` in pg.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct BufferTag {
    rnode: RelFileNode,
    block_num: BlockNumber,
}

#[derive(Debug)]
struct BufferDesc {
    tag: BufferTag,
    buf_id: Buffer, // buffer's index number (from 0)
    locked: bool,
    dirty: bool,
    valid: bool,
}

// TODO: Use RwLock to minimize RwLock scope and change ScanState to hold BufferManager ref
pub struct BufferManager {
    config: Rc<Config>,
    smgr: StorageManager,
    buffer_descriptors: Vec<BufferDesc>,
    pages: Vec<Page>,
    // Hash from BufferTag to index of descriptor and page
    // See LocalBufHash in pg.
    buffer_hash: HashMap<BufferTag, Buffer>,
}

impl Drop for BufferManager {
    fn drop(&mut self) {
        self.flush_buffers();
    }
}

macro_rules! ensure_pages_length {
    ($self:ident) => (
        if $self.pages.len() > N_BUFFERS {
            warn!("Page length ({}) exceeds N_BUFFERS ({})", $self.pages.len(), N_BUFFERS);
            // panic!("Page length ({}) exceeds N_BUFFERS ({})", self.pages.len(), N_BUFFERS);
        }
    )
}

impl BufferManager {
    pub fn new(size: usize, config: Rc<Config>) -> BufferManager {
        BufferManager {
            config: config.clone(),
            smgr: StorageManager::new(config),
            buffer_descriptors: Vec::with_capacity(N_BUFFERS),
            pages: Vec::with_capacity(N_BUFFERS),
            buffer_hash: HashMap::new(),
        }
    }

    // `heap_insert` function in pg.
    pub fn heap_insert(&mut self, relation: &RelationData, tuple: &TupleTableSlot) {
        let buffer = self.relation_get_buffer_for_tuple(relation, tuple.len());
        self.relation_put_heap_tuple(buffer, tuple);
    }

    // `heap_delete` in pg.
    pub fn heap_delete(&mut self, relation: &RelationData, tid: &ItemPointerData) {
        let block = ::tuple::item_pointer_get_block_number(tid);
        let buffer = self.read_buffer(relation, block);
        let page = self.get_page(buffer);
        let lineoff = tid.item_pointer_get_offset_number();

        let lp = page.get_item_ref(lineoff);
        let len = lp.lp_len();

        // TODO: Is this cast correct?
        let mut tuple_data = HeapTupleData::new_with_full_len(len as u32);

        debug!("Deleting record on (block: {}, lineoff: {})", block, lineoff);

        tuple_data.load_without_len(page.get_entry_pointer(lineoff).unwrap(), tid.clone());
        tuple_data.t_data.set_heap_keys_updated();
        tuple_data.write_data(page.get_entry_pointer(lineoff).unwrap() as *mut libc::c_void);
    }

    // `RelationPutHeapTuple` in pg.
    fn relation_put_heap_tuple(&mut self, buffer :Buffer, tuple: &TupleTableSlot) {
        let page = self.get_page_mut(buffer);
        page.add_tuple_slot_entry(tuple).unwrap();
    }

    // `RelationGetBufferForTuple` function in pg.
    fn relation_get_buffer_for_tuple(&mut self, relation: &RelationData, len: u32) -> Buffer {
        if (len as usize) > MAX_HEAP_TUPLE_SIZE {
            panic!("row is too big: size {}, maximum size {}", len, MAX_HEAP_TUPLE_SIZE);
        }

        let mut target_block = InvalidBlockNumber;

        {
            let mut rd_smgr = self.smgr.relation_smgropen(relation).borrow_mut();
            target_block = rd_smgr.smgr_targblock;

            if target_block == InvalidBlockNumber {
                let nblocks = rd_smgr.mdnblocks();

                if nblocks > 0 {
                    target_block = nblocks - 1;
                } else {
                    target_block = 0;
                }
            }
        }

        // When we implement VACUUM and FSM, we should check there is space
        // in each page in loop. But now we do not have such fetures, so
        // we can simply get new page if there is no space in current page.
        //
        // loop {
        {
            let buffer = self.read_buffer(relation, target_block);
            let page_free_space = self.get_page_free_space(buffer);
            let mut rd_smgr = self.smgr.relation_smgropen(&relation).borrow_mut();
            if (len as usize) <= page_free_space {
                rd_smgr.smgr_targblock = target_block;
                return buffer;
            }

            // TODO: release buffer.
        }

        // `buffer = ReadBufferBI(relation, P_NEW, bistate);` call in pg.
        let buffer = self.read_buffer_new_page(relation);
        self.get_page_mut(buffer).page_init(DEFAULT_BLOCK_SIZE);
        let block_num = self.buffer_get_block_number(buffer);
        let mut rd_smgr = self.smgr.relation_smgropen(&relation).borrow_mut();
        rd_smgr.smgr_targblock = block_num;

        return buffer
    }

    // `BufferGetBlockNumber` in pg.
    fn buffer_get_block_number(&self, buffer :Buffer) -> BlockNumber {
        let buf = unwrap_buffer_id(buffer);
        let tag = &self.buffer_descriptors[buf].tag;
        tag.block_num
    }

    // `RelationGetNumberOfBlocks` in pg.
    pub fn relation_get_number_of_blocks(&mut self, relation: &RelationData) -> BlockNumber {
        let mut rd_smgr = self.smgr.relation_smgropen(relation).borrow_mut();
        rd_smgr.mdnblocks()
    }

    // `blockNum == P_NEW` case of `ReadBuffer_common` in pg.
    //
    // This method create new page.
    fn read_buffer_new_page(&mut self, relation: &RelationData) -> Buffer {
        let tag = {
            let mut rd_smgr = self.smgr.relation_smgropen(relation).borrow_mut();
            // Get latest block number.
            let block_num = rd_smgr.mdnblocks();

            BufferTag {
                rnode: rd_smgr.smgr_rnode.clone(),
                block_num: block_num,
            }
        };

        self.read_buffer_common(relation, tag, true)
    }


    // `ReadBuffer` function in pg.
    // This should recieve Relation instead of RelFileNode because we should
    // determine which block should be loaded, but the block info is stored in
    // Relation (SMgrRelationData).
    pub fn read_buffer(&mut self, relation: &RelationData, block_num: BlockNumber) -> Buffer {
        let tag = {
            let rd_smgr = self.smgr.relation_smgropen(relation).borrow();
            BufferTag {
                rnode: rd_smgr.smgr_rnode.clone(),
                block_num: block_num,
            }
        };

        self.read_buffer_common(relation, tag, false)
    }

    // ReadBuffer_common in pg
    fn read_buffer_common(&mut self, relation: &RelationData, tag: BufferTag, new_page: bool) -> Buffer {
        {
            let opt = self.buffer_hash.get(&tag);

            if opt.is_some() {
                return opt.unwrap().clone();
            }
        }

        let buffer = Buffer::Buffer(self.pages.len());
        let descriptor = BufferDesc {
            tag: tag.clone(),
            buf_id: buffer,
            locked: false,
            dirty: false,
            valid: true,
        };

        debug!("page is pushed (len: {}, tag {:?})", self.pages.len(), tag);

        let mut rd_smgr = self.smgr.relation_smgropen(relation).borrow_mut();

        let page = if new_page {
            let mut page = Page::new(DEFAULT_BLOCK_SIZE);
            page.fill_with_zero(DEFAULT_BLOCK_SIZE as usize);
            rd_smgr.mdextend(tag.block_num, page.header_pointer());
            page
        } else {
            let page = Page::new(DEFAULT_BLOCK_SIZE);
            rd_smgr.mdread(tag.block_num, page.header_pointer());
            page
        };

        self.pages.push(page);
        self.buffer_descriptors.push(descriptor);
        self.buffer_hash.entry(tag).or_insert_with(|| {
            buffer
        });

        ensure_pages_length!(self);

        buffer
    }

    // `FlushBuffer` in pg.
    fn flush_buffer(&mut self, buffer_id: Buffer) {
        let buf = unwrap_buffer_id(buffer_id);
        self.flush_buffer_with_index(buf);
    }

    fn flush_buffer_with_index(&mut self, i: usize) {
        debug!("Page {} is flushed", i);

        let page = &self.pages[i];
        let descriptor = &self.buffer_descriptors[i];
        let rnode = &descriptor.tag.rnode;
        let block_num = descriptor.tag.block_num;

        let relation_data = self.smgr.smgropen(&rnode);
        relation_data.borrow_mut().mdwrite(block_num, page.header_pointer());
    }

    fn flush_buffers(&mut self) {
        let len = self.buffer_descriptors.len();

        for i in 0..len {
            self.flush_buffer_with_index(i);
        }
    }

    fn get_page_free_space(&self, buffer_id: Buffer) -> usize {
        self.get_page(buffer_id).page_get_free_space()
    }

    // See `BufHdrGetBlock` in pg.
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
}

