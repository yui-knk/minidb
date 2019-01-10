use catalog::mini_attribute::MiniAttributeRecord;
use ty::{Ty, load_ty};

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
struct HeapTupleData {
    t_len: u32,
    t_data: Box<HeapTupleHeaderData>,
}

// The contents of this struct are directly read from/write to
// a tuple of pages.
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

    pub fn load_data(&mut self, src: *const libc::c_void, n: u32) {
        self.heap_tuple.load(src, n);
    }

    pub fn attrs_count(&self) -> usize {
        self.tuple_desc.attrs_count()
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

    fn attrs_len(&self, index: usize) -> u32 {
        self.attrs.iter().take(index).fold(0, |acc, attr| acc + attr.len) as u32
    }
}

impl HeapTupleData {
    fn new(len: u32) -> HeapTupleData {
        let data = Box::new(HeapTupleHeaderData::new(len));

        HeapTupleData {
            t_len: len,
            t_data: data,
        }
    }

    fn load(&mut self, src: *const libc::c_void, n: u32) {
        if self.t_len < n {
            panic!("Try to load over size data. t_len: {}, n: {}.", self.t_len, n);
        }

        self.t_data.load(src, n);
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
