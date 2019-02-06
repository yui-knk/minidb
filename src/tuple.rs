use std::mem;

use catalog::mini_attribute::MiniAttributeRecord;
use off::{OffsetNumber, FirstOffsetNumber, InvalidOffsetNumber};
use ty::{TypeValue, load_type_value, build_type_value};
use buffer_manager::{BlockIdData, BlockNumber, InvalidBlockNumber};

pub struct KeyValue<'a> {
    key: &'a str,
    value: &'a str,
}

pub struct KeyValueBuilder<'a> {
    key_values: Vec<KeyValue<'a>>,
}

impl<'a> KeyValueBuilder<'a> {
    pub fn new() -> KeyValueBuilder<'a> {
        KeyValueBuilder { key_values: Vec::new() }
    }

    pub fn add_pair(&mut self, key: &'a str, value: &'a str) {
        self.key_values.push(KeyValue {
            key: key,
            value: value,
        })
    }

    pub fn build(self) -> Vec<KeyValue<'a>> {
        self.key_values
    }
}

// From itemptr.h in pg.
#[derive(Debug, Clone)]
pub struct ItemPointerData
{
    ip_blkid: BlockIdData,
    // offset number in a page (lp_off of ItemIdData).
    ip_posid: OffsetNumber,
}

// ItemPointerSetInvalid in pg.
pub fn item_pointer_set_invalid(pointer: &mut ItemPointerData) {
    pointer.ip_blkid = InvalidBlockNumber;
    pointer.ip_posid = InvalidOffsetNumber;
}

// ItemPointerSet in pg.
pub fn item_pointer_set(pointer: &mut ItemPointerData, block_number: BlockNumber, off_num: OffsetNumber) {
    pointer.ip_blkid = block_number;
    pointer.ip_posid = off_num;
}

// ItemPointerGetBlockNumber in pg.
pub fn item_pointer_get_block_number(pointer: &ItemPointerData) -> BlockIdData {
    pointer.ip_blkid
}


// Tuple means row of a table.
pub struct TupleTableSlot {
    tuple_desc: Box<TupleDesc>,
    pub heap_tuple: Box<HeapTupleData>,
}

// This manages metadata (e.g. column definitions).
struct TupleDesc {
    attrs: Vec<MiniAttributeRecord>,
}

// This manages tuple data.
#[derive(Debug)]
pub struct HeapTupleData {
    t_len: u32,
    // SelfItemPointer
    pub t_self: ItemPointerData,
    pub t_data: Box<HeapTupleHeaderData>,
}

// The contents of this struct are directly read from/write to
// a tuple of pages.
//
// The data field of this struct includes t_infomask2
// and t_infomask.
//
// struct HeapTupleHeaderData {
//     t_infomask2: u16,
//     t_infomask: u16,
//     data: *mut u8,
// }
#[derive(Debug)]
pub struct HeapTupleHeaderData {
    ptr: *mut u8,
}

// t_infomask2 and t_infomask
const SIZE_OF_HEADER_HEADER: usize = mem::size_of::<u16>() * 2;

// tuple was updated and key cols modified, or tuple deleted
const HEAP_KEYS_UPDATED: u16 = 0x2000;

impl TupleTableSlot {
    pub fn new(attrs: Vec<MiniAttributeRecord>) -> TupleTableSlot {
        let len = attrs.iter().fold(0, |acc, attr| acc + attr.len) as u32;
        let tuple_desc = TupleDesc::new(attrs);
        let heap_tuple = HeapTupleData::new(len);

        TupleTableSlot {
            tuple_desc: Box::new(tuple_desc),
            heap_tuple: Box::new(heap_tuple),
        }
    }

    pub fn len(&self) -> u32 {
        self.heap_tuple.t_len
    }

    pub fn tid(&self) -> &ItemPointerData {
        &self.heap_tuple.t_self
    }

    pub fn load_data(&mut self, src: *const libc::c_void, n: u32, t_self: ItemPointerData) {
        self.heap_tuple.load(src, n, t_self);
    }

    pub fn load_data_without_len(&mut self, src: *const libc::c_void, t_self: ItemPointerData) {
        self.heap_tuple.load_without_len(src, t_self);
    }

    pub fn attrs_count(&self) -> usize {
        self.tuple_desc.attrs_count()
    }

    pub fn get_index_from_name(&self, name: &str) -> usize {
        self.tuple_desc.attrs.iter().position(|ref a| a.name == name).unwrap()
    }

    // index is 0-origin.
    pub fn get_column(&self, index: usize) -> Box<TypeValue> {
        self.check_index(index);

        let attr = &self.tuple_desc.attrs[index];
        let ptr = self.attr_ptr(index) as *const libc::c_void;
        load_type_value(&attr.ty, ptr)
    }

    pub fn set_column(&mut self, index: usize, ty: &TypeValue) {
        // TODO: we should also check type of passed `ty` matches with
        //       the type of `attr`.
        self.check_index(index);

        let src = ty.as_pointer();
        let n = ty.len();
        let offset = self.tuple_desc.attrs_len(index) as usize;
        self.heap_tuple.t_data.set_column(src, n, offset);
    }

    pub fn update_tuple(&mut self, key_values: Vec<KeyValue>) -> Result<(), String> {
        if self.attrs_count() != key_values.len() {
            return Err(format!("Length not match. attrs: {}, key_values: {}", self.attrs_count(), key_values.len()));
        }

        for (kv, attr) in key_values.iter().zip(self.tuple_desc.attrs.iter()) {
            if kv.key != attr.name {
                return Err(format!("Name not match. attrs: {}, key_values: {}", attr.name, kv.key));
            }
        }

        let v: Vec<Box<TypeValue>> = key_values.iter().zip(self.tuple_desc.attrs.iter()).map(|(kv, attr)|
            build_type_value(&attr.ty, &kv.value)
        ).collect();

        for (i, t) in v.iter().enumerate() {
            self.set_column(i, t.as_ref());
        }

        Ok(())
    }

    pub fn data_ptr(&self) -> *const libc::c_void {
        self.heap_tuple.data_ptr()
    }

    fn check_index(&self, index: usize) {
        if !(index < self.attrs_count()) {
            panic!("Out of index. attrs_count: {}, index: {}", self.attrs_count(), index);
        }
    }

    fn attr_ptr(&self, index: usize) -> *const u8 {
        unsafe {
            let p = self.heap_tuple.t_data.data_ptr() as *const u8;
            p.add(self.tuple_desc.attrs_len(index) as usize)
        }
    }
}

impl TupleDesc {
    fn new(attrs: Vec<MiniAttributeRecord>) -> TupleDesc {
        TupleDesc {
            attrs: attrs,
        }
    }

    fn attrs_count(&self) -> usize {
        self.attrs.len()
    }

    fn attrs_total_len(&self) -> u32 {
        self.attrs.iter().fold(0, |acc, attr| acc + attr.len) as u32
    }

    fn attrs_len(&self, index: usize) -> u32 {
        self.attrs.iter().take(index).fold(0, |acc, attr| acc + attr.len) as u32
    }
}

impl HeapTupleData {
    pub fn new(data_len: u32) -> HeapTupleData {
        let len = data_len + SIZE_OF_HEADER_HEADER as u32;
        let data = Box::new(HeapTupleHeaderData::new(len));

        HeapTupleData {
            t_len: len,
            t_self: ItemPointerData::new(),
            t_data: data,
        }
    }

    pub fn new_with_full_len(len: u32) -> HeapTupleData {
        let data = Box::new(HeapTupleHeaderData::new(len));

        HeapTupleData {
            t_len: len,
            t_self: ItemPointerData::new(),
            t_data: data,
        }
    }

    fn load(&mut self, src: *const libc::c_void, n: u32, t_self: ItemPointerData) {
        if self.t_len < n {
            panic!("Try to load over size data. t_len: {}, n: {}.", self.t_len, n);
        }

        self.t_self = t_self;
        self.t_data.load(src, n);
    }

    pub fn load_without_len(&mut self, src: *const libc::c_void, t_self: ItemPointerData) {
        self.t_self = t_self;
        self.t_data.load(src, self.t_len);
    }

    pub fn get_item_offset_number(&self) -> OffsetNumber {
        self.t_self.ip_posid
    }

    pub fn data_ptr(&self) -> *const libc::c_void {
        self.t_data.ptr as *const libc::c_void
    }

    pub fn write_data(&mut self, dest: *mut libc::c_void) {
        self.t_data.write_data(dest, self.t_len);
    }
}

impl HeapTupleHeaderData {
    fn new(data_size: u32) -> HeapTupleHeaderData {
        unsafe {
            let data_p: *mut u8 = libc::malloc(data_size as libc::size_t) as *mut u8;

            debug!("HeapTupleHeaderData malloc: {:?}, {}", data_p, data_size);

            if data_p.is_null() {
                panic!("failed to allocate memory");
            }

            HeapTupleHeaderData {
                ptr: data_p
            }
        }
    }

    pub fn set_heap_keys_updated(&mut self) {
        let mask2 = self.t_infomask2();
        self.set_t_infomask2(mask2 | HEAP_KEYS_UPDATED);
    }

    pub fn heap_keys_updated_p(&self) -> bool {
        let mask2 = self.t_infomask2();
        (mask2 & HEAP_KEYS_UPDATED) != 0
    }

    fn t_infomask2(&self) -> u16 {
        unsafe {
            let p = self.ptr as *const u16;
            *p
        }
    }

    fn set_t_infomask2(&mut self, mask: u16) {
        unsafe {
            let p = self.ptr as *mut u16;
            *p = mask;
        }
    }

    fn data_ptr(&self) -> *const libc::c_void {
        unsafe {
            self.ptr.add(SIZE_OF_HEADER_HEADER) as *const libc::c_void
        }
    }

    fn load(&mut self, src: *const libc::c_void, n: u32) {
        unsafe {
            libc::memcpy(self.ptr as *mut libc::c_void, src, n as usize);

            debug!("HeapTupleHeaderData load: {:?}, {:?}", self.ptr, n);
        }
    }

    fn set_column(&mut self, src: *const libc::c_void, n: u32, offset: usize) {
        unsafe {
            let dest: *mut libc::c_void = self.data_ptr().add(offset) as *mut libc::c_void;
            libc::memcpy(dest, src, n as usize);

            debug!("HeapTupleHeaderData set_column: {:?}, {:?}", dest, n);
        }
    }

    fn write_data(&mut self, dest: *mut libc::c_void, n: u32) {
        unsafe {
            libc::memcpy(dest, self.ptr as *const libc::c_void, n as usize);

            debug!("HeapTupleHeaderData write_data: {:?}, {:?}", self.ptr, n);
        }
    }
}

impl Drop for HeapTupleHeaderData {
    fn drop(&mut self) {
        unsafe {
            if self.ptr.is_null() {
                panic!("ptr should not be null pointer.");
            }

            debug!("HeapTupleHeaderData free: {:?}", self.ptr);

            libc::free(self.ptr as *mut libc::c_void);
        }
    }
}

impl ItemPointerData {
    pub fn new() -> ItemPointerData {
        let ip_blkid = 0;
        let ip_posid = FirstOffsetNumber;

        ItemPointerData {
            ip_blkid: ip_blkid,
            ip_posid: ip_posid,
        }
    }

    // ItemPointerGetOffsetNumber in pg
    pub fn item_pointer_get_offset_number(&self) -> OffsetNumber {
        self.ip_posid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::mini_attribute::TypeLabel;
    use ty::Integer;

    #[test]
    fn test_tuple_table_slot() {
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
        let slot = TupleTableSlot::new(attrs);

        assert_eq!(slot.attrs_count(), 2);
        assert_eq!(slot.attr_ptr(0), slot.heap_tuple.t_data.data_ptr() as *const u8);
        unsafe {
            assert_eq!(slot.attr_ptr(1), (slot.heap_tuple.t_data.data_ptr() as *const u8).add(4));
        }
    }

    #[test]
    fn test_tuple_table_slot_get_set_column() {
        let mut attrs = Vec::new();
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            20001,
            20002,
            TypeLabel::Integer,
            4
        ));
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            20003,
            20004,
            TypeLabel::Integer,
            4
        ));
        let mut slot = TupleTableSlot::new(attrs);

        slot.set_column(0, &Integer { elem: 10 });
        slot.set_column(1, &Integer { elem: 22 });

        assert_eq!(slot.get_column(0).as_string(), "10".to_string());
        assert_eq!(slot.get_column(1).as_string(), "22".to_string());
    }
}
