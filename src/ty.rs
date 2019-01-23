// Column type

use std::slice;
use std::io::{Write};
use byteorder::{WriteBytesExt, ReadBytesExt};

use catalog::mini_attribute::{TypeLabel, ty_byte_len};

pub trait TypeValue {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()>;
    fn len(&self) -> u32;
    fn as_string(&self) -> String;
    fn as_pointer(&self) -> *const libc::c_void;
}

// This function transforms data in database to TypeValue, so use methods of
// byteorder crate.
pub fn load_type_value(tl: &TypeLabel, src: *const libc::c_void) -> Box<TypeValue> {
    let len = ty_byte_len(tl);

    match tl {
        TypeLabel::Integer => {
            let ptr: *const u8 = src as *const u8;
            let mut s = unsafe { slice::from_raw_parts(ptr, len as usize) };
            let i = s.read_i32::<byteorder::LittleEndian>().unwrap();
            Box::new(Integer { elem: i })
        }
    }
}

// This function transforms input from user, mainly SQL, to
// TypeValue, so use `parse` method. We can call `unwrap` because
// `row` will be passed parser syntax check.
pub fn build_type_value(tl: &TypeLabel, row: &str) -> Box<TypeValue> {
    match tl {
        TypeLabel::Integer => {
            let elem = row.parse::<i32>().unwrap();
            Box::new(Integer { elem: elem })
        }
    }
}

// Signed 4 bytes integer
pub struct Integer {
    pub elem: i32,
}

impl TypeValue for Integer {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()> {
        wrt.write_i32::<byteorder::LittleEndian>(self.elem)
    }

    fn len(&self) -> u32 {
        4
    }

    fn as_string(&self) -> String {
        self.elem.to_string()
    }

    fn as_pointer(&self) -> *const libc::c_void {
        let p: *const i32 = &self.elem;
        p as *const libc::c_void
    }
}
