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

    pub fn is_empty(&self) -> bool {
        self.header().pd_lower as usize <= HEADER_BYTE_SIZE
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
    }

    #[test]
    fn test_page_empty() {
        let page = Page::new(DEFAULT_BLOCK_SIZE);

        assert_eq!(page.is_empty(), true);
    }
}

