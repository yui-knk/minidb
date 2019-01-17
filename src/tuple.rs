use catalog::mini_attribute::MiniAttributeRecord;
use off::{OffsetNumber, FirstOffsetNumber};
use ty::{Ty, load_ty};
use page::{ItemIdData};

// Form itemptr.h in pg.
#[derive(Debug, Clone)]
pub struct ItemPointerData
{
    // Memo: BlockIdData in pg.
    pub ip_blkid: ItemIdData,
    pub ip_posid: OffsetNumber,
}


// Tuple means row of a table.
pub struct TupleTableSlot {
    tuple_desc: Box<TupleDesc>,
    heap_tuple: Box<HeapTupleData>,
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
    t_data: Box<HeapTupleHeaderData>,
}

// The contents of this struct are directly read from/write to
// a tuple of pages.
#[derive(Debug)]
struct HeapTupleHeaderData {
    data: *mut u8,
}

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

    pub fn load_data(&mut self, src: *const libc::c_void, n: u32, t_self: ItemPointerData) {
        self.heap_tuple.load(src, n, t_self);
    }

    pub fn load_data_without_len(&mut self, src: *const libc::c_void, t_self: ItemPointerData) {
        self.heap_tuple.load_without_len(src, t_self);
    }

    pub fn attrs_count(&self) -> usize {
        self.tuple_desc.attrs_count()
    }

    pub fn attrs_total_len(&self) -> u32 {
        self.tuple_desc.attrs_total_len()
    }

    // index is 0-origin.
    pub fn get_column(&self, index: usize) -> Box<Ty> {
        self.check_index(index);

        let attr = &self.tuple_desc.attrs[index];
        let ptr = self.attr_ptr(index) as *const libc::c_void;
        load_ty(attr.type_name.as_str(), ptr, 4).unwrap()
    }

    pub fn set_column(&mut self, index: usize, ty: &Ty) {
        // TODO: we should also check type of passed `ty` matches with
        //       the type of `attr`.
        self.check_index(index);

        let src = ty.as_pointer();
        let n = ty.len();
        let offset = self.tuple_desc.attrs_len(index) as usize;
        self.heap_tuple.t_data.set_column(src, n, offset);
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
            self.heap_tuple.t_data.data.add(self.tuple_desc.attrs_len(index) as usize)
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
    pub fn new(len: u32) -> HeapTupleData {
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
        self.t_data.data as *const libc::c_void
    }
}

impl HeapTupleHeaderData {
    fn new(data_size: u32) -> HeapTupleHeaderData {
        unsafe {
            let data_p: *mut u8 = libc::malloc(data_size as libc::size_t) as *mut u8;

            if data_p.is_null() {
                panic!("failed to allocate memory");
            }

            HeapTupleHeaderData { data: data_p }
        }
    }

    fn load(&mut self, src: *const libc::c_void, n: u32) {
        unsafe {
            libc::memcpy(self.data as *mut libc::c_void, src, n as usize);
        }
    }

    fn set_column(&mut self, src: *const libc::c_void, n: u32, offset: usize) {
        unsafe {
            let dest: *mut libc::c_void = self.data.add(offset) as *mut libc::c_void;
            libc::memcpy(dest, src, n as usize);
        }
    }
}

impl Drop for HeapTupleHeaderData {
    fn drop(&mut self) {
        unsafe {
            if self.data.is_null() {
                panic!("data should not be null pointer.");
            }

            libc::free(self.data as *mut libc::c_void);
        }
    }
}

impl ItemPointerData {
    pub fn new() -> ItemPointerData {
        let ip_blkid = ItemIdData::new(0);
        let ip_posid = FirstOffsetNumber;

        ItemPointerData {
            ip_blkid: ip_blkid,
            ip_posid: ip_posid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ty::Integer;

    #[test]
    fn test_tuple_table_slot() {
        let mut attrs = Vec::new();
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            "dbname".to_string(),
            "class_name".to_string(),
            "integer".to_string(),
            4
        ));
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            "dbname".to_string(),
            "class_name".to_string(),
            "integer".to_string(),
            4
        ));
        let slot = TupleTableSlot::new(attrs);

        assert_eq!(slot.attrs_count(), 2);
        assert_eq!(slot.attr_ptr(0), slot.heap_tuple.t_data.data);
        unsafe {
            assert_eq!(slot.attr_ptr(1), slot.heap_tuple.t_data.data.add(4));
        }
    }

    #[test]
    fn test_tuple_table_slot_get_set_column() {
        let mut attrs = Vec::new();
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            "dbname".to_string(),
            "class_name".to_string(),
            "integer".to_string(),
            4
        ));
        attrs.push(MiniAttributeRecord::new(
            "name".to_string(),
            "dbname".to_string(),
            "class_name".to_string(),
            "integer".to_string(),
            4
        ));
        let mut slot = TupleTableSlot::new(attrs);

        slot.set_column(0, &Integer { elem: 10 });
        slot.set_column(1, &Integer { elem: 22 });

        assert_eq!(slot.get_column(0).as_string(), "10".to_string());
        assert_eq!(slot.get_column(1).as_string(), "22".to_string());
    }
}
