// Column type

use std::io::{Write};
use byteorder::{LittleEndian, WriteBytesExt};

use catalog::mini_attribute::TypeLabel;

pub trait TypeValue {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()>;
    fn len(&self) -> u32;
    fn as_string(&self) -> String;
    fn as_pointer(&self) -> *const libc::c_void;
}

pub fn load_ty(tl: &TypeLabel, src: *const libc::c_void) -> Result<Box<TypeValue>, String> {
    match tl {
        TypeLabel::Integer => {
            unsafe {
                let mut i = Integer { elem: 0 };
                let elem_p: *mut i32 = &mut i.elem;
                *elem_p = *(src as *const i32);
                Ok(Box::new(i))
            }
        }
    }
}

pub fn build_ty(tl: &TypeLabel, row: &str) -> Result<Box<TypeValue>, String> {
    match tl {
        TypeLabel::Integer => {
            let elem = row.parse::<i32>().unwrap();
            Ok(Box::new(Integer { elem: elem }))
        }
    }
}

// Signed 4 bytes integer
pub struct Integer {
    pub elem: i32,
}

impl TypeValue for Integer {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()> {
        wrt.write_i32::<LittleEndian>(self.elem)
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
