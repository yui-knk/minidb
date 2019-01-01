use std::mem;

type LocationIndex = u16;

pub struct PageMetaData {

}

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
    header: *mut Header,
}

// This is struct for line pointer.
//
// 32bit is used separately:
// (1) 15bit (lp_off) is offset to tuple (from start of page)
// (2)  2bit (lp_flags) is state of item pointer, see below
// (3) 15bit (lp_len) is byte length of tuple
pub struct ItemIdData {
    data: u32
}

const MEM_SIZE_OF_U8: usize = mem::size_of::<u8>();
const MEM_SIZE_OF_U8_AS_U16: u16 = mem::size_of::<u8>() as u16;

const ITEM_ID_DATA_BYTE_SIZE: usize = mem::size_of::<ItemIdData>();

// see: PageHeaderData
pub struct Header {
    // offset to start of free space
    pd_lower: LocationIndex,
    // offset to end of free space
    pd_upper: LocationIndex,
}

const HEADER_BYTE_SIZE: usize = mem::size_of::<Header>();

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

impl Header {
    pub fn new(block_size: u16) -> Header {
        Header {
            pd_lower: HEADER_BYTE_SIZE as u16,
            pd_upper: block_size,
        }
    }

    pub fn init(header: &mut Header, block_size: u16) {
        header.pd_lower = HEADER_BYTE_SIZE as u16;
        header.pd_upper = block_size;
    }

    pub fn from_bytes(buf: &[u8]) -> Header {
        if buf.len() != HEADER_BYTE_SIZE {
            panic!("Length of from_bytes should be {}, but {}.", HEADER_BYTE_SIZE, buf.len());
        }

        Header {
            pd_lower: ((buf[0] as u16) << 8) | buf[1] as u16,
            pd_upper: ((buf[2] as u16) << 8) | buf[4] as u16,
        }
    }
}

impl Page {
    pub fn new(block_size: u16) -> Page {
        unsafe {
            let header_p: *mut Header = libc::malloc(block_size as libc::size_t) as *mut Header;

            if header_p.is_null() {
                panic!("failed to allocate memory");
            }

            Header::init(&mut *header_p, block_size);
            Page { header: header_p }
        }
    }

    pub fn header(&self) -> &Header {
        unsafe {
            if self.header.is_null() {
                panic!("header should not be null pointer.");
            }

            &*self.header
        }
    }

    pub fn mut_header(&mut self) -> &mut Header {
        unsafe {
            if self.header.is_null() {
                panic!("header should not be null pointer.");
            }

            &mut *self.header
        }
    }

    pub fn is_empty(&self) -> bool {
        self.header().pd_lower as usize <= HEADER_BYTE_SIZE
    }

    pub fn add_entry(&mut self, entry: &Vec<u8>) -> Result<(), String> {
        let len = (entry.len() * MEM_SIZE_OF_U8) as u16;

        if self.has_space(len) {
            self.mut_header().pd_upper -= len;
            let item = ItemIdData::new_with_lps(self.header().pd_upper, 0, len);

            unsafe {
                let item_p: *mut ItemIdData = self.header.add(self.header().pd_lower as usize) as *mut ItemIdData;
                *item_p = item;

                for i in 0..len {
                    let off = self.mut_header().pd_upper;
                    let p: *mut u8 = self.header.add((off + MEM_SIZE_OF_U8_AS_U16 * i) as usize) as *mut u8;
                    *p = entry[i as usize];
                }
            }

            self.mut_header().pd_lower += ITEM_ID_DATA_BYTE_SIZE as u16;
            Ok(())
        } else {
            Err(format!("Does not have enough space for {}", len))
        }
    }

    fn has_space(&self, len: u16) -> bool {
        self.header().pd_lower <= (self.header().pd_upper - len)
    }

    fn entry_count(&self) -> u16 {
        (((self.header().pd_lower as usize) - HEADER_BYTE_SIZE) / ITEM_ID_DATA_BYTE_SIZE) as u16
    }

    fn get_item(&self, index: u16) -> &ItemIdData {
        unsafe {
            &*(self.header.add(HEADER_BYTE_SIZE + ITEM_ID_DATA_BYTE_SIZE * (index as usize)) as *const ItemIdData)
        }
    }

    // index is 0-origin.
    fn get_entry(&self, index: u16) -> Result<Vec<u8>, String> {
        if index > self.entry_count() {
            return Err(format!("Index over entry_count. index: {}, entry_count: {}", index, self.entry_count()));
        }

        unsafe {
            let item_p: *const ItemIdData = self.get_item(index);
            let off = (*item_p).lp_off();
            let len = (*item_p).lp_len();
            let mut v = Vec::with_capacity(len as usize);

            for i in 0..len {
                let p: *const u8 = self.header.add((off + MEM_SIZE_OF_U8_AS_U16 * i) as usize) as *const u8;
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
    use config::DEFAULT_BLOCK_SIZE;

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
    fn test_add_entry() {
        let mut page = Page::new(DEFAULT_BLOCK_SIZE);
        let entry1: Vec<u8> = vec![1, 2, 3];
        let entry2: Vec<u8> = vec![3, 2, 1, 0];
        let entry_size1 = (mem::size_of::<u8>() * 3) as u16;
        let entry_size2 = (mem::size_of::<u8>() * 4) as u16;

        page.add_entry(&entry1).unwrap();
        assert_eq!(page.header().pd_lower, (HEADER_BYTE_SIZE + ITEM_ID_DATA_BYTE_SIZE) as u16);
        assert_eq!(page.header().pd_upper, DEFAULT_BLOCK_SIZE - entry_size1);
        assert_eq!(page.is_empty(), false);
        assert_eq!(page.entry_count(), 1);
        assert_eq!(page.get_item(0).lp_len(), 3);
        assert_eq!(page.get_entry(0).unwrap(), entry1);

        page.add_entry(&entry2).unwrap();
        assert_eq!(page.header().pd_lower, (HEADER_BYTE_SIZE + ITEM_ID_DATA_BYTE_SIZE * 2) as u16);
        assert_eq!(page.header().pd_upper, DEFAULT_BLOCK_SIZE - entry_size1 - entry_size2);
        assert_eq!(page.is_empty(), false);
        assert_eq!(page.entry_count(), 2);
        assert_eq!(page.get_item(1).lp_len(), 4);
        assert_eq!(page.get_entry(1).unwrap(), entry2);
    }
}
