use std::mem;

type LocationIndex = u16;

pub struct Page {

}

// 32bit is used separately
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
}

impl Header {
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


