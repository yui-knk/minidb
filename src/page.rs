use std::mem;

use config::DEFAULT_BLOCK_SIZE;
use tuple::{TupleTableSlot};
use off::{OffsetNumber};

type LocationIndex = u16;

// +----------------+---------------------------------+
// | PageHeaderData | linp1 linp2 linp3 ...           |
// +-----------+----+---------------------------------+
// | ... linpN |                                      |
// +-----------+--------------------------------------+
// |           ^ pd_lower                             |
// |                                                  |
// |             v pd_upper                           |
// +-------------+------------------------------------+
// |             | tupleN ...                         |
// +-------------+------------------+-----------------+
// |       ... tuple3 tuple2 tuple1 | "special space" |
// +--------------------------------+-----------------+
//                                  ^ pd_special

// We malloc block_size memory for header, lines and tuples.
pub struct Page {
    header: *mut PageHeaderData,
}

// This is struct for line pointer.
//
// 32bit is used separately:
// (1) 15bit (lp_off) is offset to tuple (from start of page)
// (2)  2bit (lp_flags) is state of item pointer, see below
// (3) 15bit (lp_len) is byte length of tuple
#[derive(Debug, Clone, Copy)]
pub struct ItemIdData {
    data: u32
}

const MEM_SIZE_OF_U8: usize = mem::size_of::<u8>();
const MEM_SIZE_OF_U8_AS_U16: u16 = mem::size_of::<u8>() as u16;

const ITEM_ID_DATA_BYTE_SIZE: usize = mem::size_of::<ItemIdData>();

// `PageHeaderData` in pg.
pub struct PageHeaderData {
    // offset to start of free space
    pd_lower: LocationIndex,
    // offset to end of free space
    pd_upper: LocationIndex,
}

// `SizeOfPageHeaderData` in pg.
const SIZE_OF_PAGE_HEADER_DATA: usize = mem::size_of::<PageHeaderData>();

// `MaxHeapTupleSize` in pg.
pub const MAX_HEAP_TUPLE_SIZE: usize = (DEFAULT_BLOCK_SIZE as usize) - SIZE_OF_PAGE_HEADER_DATA;

impl ItemIdData {
    pub fn new(data: u32) -> ItemIdData {
        ItemIdData { data: data }
    }

    pub fn new_with_lps(off: u16, flags: u8, len: u16) -> ItemIdData {
        let mut item = ItemIdData::new(0);
        item.set_lp_off(off);
        item.set_lp_flags(flags);
        item.set_lp_len(len);
        item
    }

    pub fn lp_off(&self) -> u16 {
        ((self.data & 0xfffe0000) >> 17) as u16
    }

    pub fn lp_flags(&self) -> u8 {
        ((self.data & 0x00018000) >> 15) as u8
    }

    pub fn lp_len(&self) -> u16 {
        ((self.data & 0x00007fff)) as u16
    }

    pub fn set_lp_off(&mut self, off: u16) {
        self.data = (self.data & !0xfffe0000) | ((off as u32) << 17);
    }

    pub fn set_lp_flags(&mut self, flags: u8) {
        self.data = (self.data & !0x00018000) | (((flags & 0x0003) as u32) << 15);
    }

    pub fn set_lp_len(&mut self, len: u16) {
        self.data = (self.data & !0x00007fff) | ((len & 0x7fff) as u32);
    }
}

impl PageHeaderData {
    fn new(block_size: u16) -> PageHeaderData {
        PageHeaderData {
            pd_lower: SIZE_OF_PAGE_HEADER_DATA as u16,
            pd_upper: block_size,
        }
    }

    fn init(&mut self, block_size: u16) {
        self.pd_lower = SIZE_OF_PAGE_HEADER_DATA as u16;
        self.pd_upper = block_size;
    }

    fn from_bytes(buf: &[u8]) -> PageHeaderData {
        if buf.len() != SIZE_OF_PAGE_HEADER_DATA {
            panic!("Length of from_bytes should be {}, but {}.", SIZE_OF_PAGE_HEADER_DATA, buf.len());
        }

        PageHeaderData {
            pd_lower: ((buf[0] as u16) << 8) | buf[1] as u16,
            pd_upper: ((buf[2] as u16) << 8) | buf[4] as u16,
        }
    }
}

impl Page {
    pub fn new(block_size: u16) -> Page {
        unsafe {
            let header_p: *mut PageHeaderData = libc::malloc(block_size as libc::size_t) as *mut PageHeaderData;

            if header_p.is_null() {
                panic!("failed to allocate memory");
            }

            (*header_p).init(block_size);
            Page { header: header_p }
        }
    }

    // `PageInit` in pg.
    pub fn page_init(&mut self, block_size: u16) {
        self.mut_header().init(block_size);
    }

    pub fn header_pointer(&self) -> *mut libc::c_void {
        self.header as *mut libc::c_void
    }

    pub fn header(&self) -> &PageHeaderData {
        unsafe {
            if self.header.is_null() {
                panic!("header should not be null pointer.");
            }

            &*self.header
        }
    }

    pub fn mut_header(&mut self) -> &mut PageHeaderData {
        unsafe {
            if self.header.is_null() {
                panic!("header should not be null pointer.");
            }

            &mut *self.header
        }
    }

    pub fn is_empty(&self) -> bool {
        self.header().pd_lower as usize <= SIZE_OF_PAGE_HEADER_DATA
    }

    pub fn page_get_max_offset_number(&self) -> OffsetNumber {
        let lower = self.header().pd_lower as usize;

        if lower <= SIZE_OF_PAGE_HEADER_DATA {
            0
        } else {
            ((lower - SIZE_OF_PAGE_HEADER_DATA) / ITEM_ID_DATA_BYTE_SIZE) as OffsetNumber
        }
    }

    pub fn add_entry(&mut self, src: *const libc::c_void, n: u16) -> Result<(), String> {
        if self.has_space(n) {
            self.mut_header().pd_upper -= n;
            let item = ItemIdData::new_with_lps(self.header().pd_upper, 0, n);

            unsafe {
                let item_p: *mut ItemIdData = (self.header as *const u8).add(self.header().pd_lower as usize) as *mut ItemIdData;
                let tuple_head_p: *mut libc::c_void = (self.header as *const u8).add(self.header().pd_upper as usize) as *mut libc::c_void;
                *item_p = item;
                libc::memcpy(tuple_head_p, src, n as usize);
            }

            self.mut_header().pd_lower += ITEM_ID_DATA_BYTE_SIZE as u16;
            Ok(())
        } else {
            Err(format!("Does not have enough space for {}", n))
        }
    }

    pub fn add_tuple_slot_entry(&mut self, slot: &TupleTableSlot) -> Result<(), String> {
        self.add_entry(slot.data_ptr(), slot.attrs_total_len() as u16)
    }

    pub fn add_vec_entry(&mut self, entry: &Vec<u8>) -> Result<(), String> {
        self.add_entry(entry.as_ptr() as *const libc::c_void, entry.len() as u16)
    }

    pub fn fill_with_zero(&mut self, n: usize) {
        unsafe {
            libc::memset(self.header as *mut libc::c_void, 0, n);
        }
    }

    fn has_space(&self, len: u16) -> bool {
        (len as usize) <= self.page_get_free_space()
    }

    // `PageGetFreeSpace` in pd.
    // See also `PageGetHeapFreeSpace` in pd.
    pub fn page_get_free_space(&self) -> usize {
        let space = (self.header().pd_upper - self.header().pd_lower) as usize;

        if space < ITEM_ID_DATA_BYTE_SIZE {
            0
        } else {
            space - ITEM_ID_DATA_BYTE_SIZE
        }
    }

    pub fn entry_count(&self) -> u16 {
        (((self.header().pd_lower as usize) - SIZE_OF_PAGE_HEADER_DATA) / ITEM_ID_DATA_BYTE_SIZE) as u16
    }

    pub fn print_info(&self) {
        println!(
            "entry_count: {}.\npd_lower: {}.\npd_upper: {}.\nSIZE_OF_PAGE_HEADER_DATA: {}.\nITEM_ID_DATA_BYTE_SIZE: {}.",
            self.entry_count(),
            self.header().pd_lower,
            self.header().pd_upper,
            SIZE_OF_PAGE_HEADER_DATA,
            ITEM_ID_DATA_BYTE_SIZE
        );
    }

    pub fn get_item_ref(&self, index: u16) -> &ItemIdData {
        unsafe {
            &*((self.header as *const u8).add(SIZE_OF_PAGE_HEADER_DATA + ITEM_ID_DATA_BYTE_SIZE * (index as usize)) as *const ItemIdData)
        }
    }

    pub fn get_item(&self, index: u16) -> ItemIdData {
        unsafe {
            *((self.header as *const u8).add(SIZE_OF_PAGE_HEADER_DATA + ITEM_ID_DATA_BYTE_SIZE * (index as usize)) as *const ItemIdData)
        }
    }

    // index is 0-origin.
    pub fn get_entry_pointer(&self, index: u16) -> Result<*const libc::c_void, String> {
        if index >= self.entry_count() {
            return Err(format!("Index over entry_count. index: {}, entry_count: {}", index, self.entry_count()));
        }

        unsafe {
            let item_p: *const ItemIdData = self.get_item_ref(index);
            let off = (*item_p).lp_off();
            let p: *const libc::c_void = (self.header as *const u8).add(off as usize) as *const libc::c_void;

            Ok(p)
        }
    }

    // index is 0-origin.
    pub fn get_entry(&self, index: u16) -> Result<Vec<u8>, String> {
        if index >= self.entry_count() {
            return Err(format!("Index over entry_count. index: {}, entry_count: {}", index, self.entry_count()));
        }

        unsafe {
            let item_p: *const ItemIdData = self.get_item_ref(index);
            let off = (*item_p).lp_off();
            let len = (*item_p).lp_len();
            let mut v = Vec::with_capacity(len as usize / MEM_SIZE_OF_U8);

            for i in 0..len {
                let p: *const u8 = (self.header as *const u8).add((off + MEM_SIZE_OF_U8_AS_U16 * i) as usize) as *const u8;
                v.push(*p);
            }

            Ok(v)
        }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        unsafe {
            if self.header.is_null() {
                panic!("header should not be null pointer.");
            }

            libc::free(self.header as *mut libc::c_void);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::mini_attribute::{MiniAttributeRecord, TypeLabel};
    use ty::Integer;

    #[test]
    fn test_item_id_data_lps() {
        let mut header1 = ItemIdData::new(0x00000000);
        header1.set_lp_off(256);
        header1.set_lp_flags(3);
        header1.set_lp_len(100);

        assert_eq!(header1.lp_off(), 256);
        assert_eq!(header1.lp_flags(), 3);
        assert_eq!(header1.lp_len(), 100);

        let mut header2 = ItemIdData::new(0xffffffff);
        header2.set_lp_off(256);
        header2.set_lp_flags(3);
        header2.set_lp_len(100);

        assert_eq!(header2.lp_off(), 256);
        assert_eq!(header2.lp_flags(), 3);
        assert_eq!(header2.lp_len(), 100);
    }

    #[test]
    fn test_page_new() {
        let page = Page::new(DEFAULT_BLOCK_SIZE);

        // Check pd_lower by is_empty method.
        assert_eq!(page.is_empty(), true);
        assert_eq!(page.header().pd_upper, DEFAULT_BLOCK_SIZE);
        assert_eq!(page.entry_count(), 0);
    }

    #[test]
    fn test_page_empty() {
        let page = Page::new(DEFAULT_BLOCK_SIZE);

        assert_eq!(page.is_empty(), true);
    }

    #[test]
    fn test_add_vec_entry() {
        let mut page = Page::new(DEFAULT_BLOCK_SIZE);
        let entry1: Vec<u8> = vec![1, 2, 3];
        let entry2: Vec<u8> = vec![3, 2, 1, 0];
        let entry_size1 = (mem::size_of::<u8>() * 3) as u16;
        let entry_size2 = (mem::size_of::<u8>() * 4) as u16;

        page.add_vec_entry(&entry1).unwrap();
        assert_eq!(page.header().pd_lower, (SIZE_OF_PAGE_HEADER_DATA + ITEM_ID_DATA_BYTE_SIZE) as u16);
        assert_eq!(page.header().pd_upper, DEFAULT_BLOCK_SIZE - entry_size1);
        assert_eq!(page.is_empty(), false);
        assert_eq!(page.entry_count(), 1);
        assert_eq!(page.get_item_ref(0).lp_len(), 3);
        assert_eq!(page.get_entry(0).unwrap(), entry1);

        page.add_vec_entry(&entry2).unwrap();
        assert_eq!(page.header().pd_lower, (SIZE_OF_PAGE_HEADER_DATA + ITEM_ID_DATA_BYTE_SIZE * 2) as u16);
        assert_eq!(page.header().pd_upper, DEFAULT_BLOCK_SIZE - entry_size1 - entry_size2);
        assert_eq!(page.is_empty(), false);
        assert_eq!(page.entry_count(), 2);
        assert_eq!(page.get_item_ref(1).lp_len(), 4);
        assert_eq!(page.get_entry(1).unwrap(), entry2);
    }

    #[test]
    fn test_add_tuple_slot_entry() {
        let mut page = Page::new(DEFAULT_BLOCK_SIZE);
        let mut attrs = Vec::new();
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            10001,
            10002,
            TypeLabel::Integer,
            4
        ));
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            10003,
            10004,
            TypeLabel::Integer,
            4
        ));
        let mut slot = TupleTableSlot::new(attrs);
        let slot_data_size = (mem::size_of::<i32>() * 2) as u16;

        slot.set_column(0, &Integer { elem: 10 });
        slot.set_column(1, &Integer { elem: 22 });
        page.add_tuple_slot_entry(&slot).unwrap();

        assert_eq!(page.header().pd_lower, (SIZE_OF_PAGE_HEADER_DATA + ITEM_ID_DATA_BYTE_SIZE * 1) as u16);
        assert_eq!(page.header().pd_upper, DEFAULT_BLOCK_SIZE - slot_data_size);
        assert_eq!(page.is_empty(), false);
        assert_eq!(page.entry_count(), 1);
        assert_eq!(page.get_item_ref(0).lp_len(), 4 * 2);
    }
}
