// Column type

use std::io::{Write};
use byteorder::{LittleEndian, WriteBytesExt};

pub trait Ty {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()>;
    fn len(&self) -> u32;
    fn as_string(&self) -> String;
    fn as_pointer(&self) -> *const libc::c_void;
}

pub fn load_ty(type_name: &str, src: *const libc::c_void, n: u32) -> Result<Box<Ty>, String> {
    match type_name {
        "integer" => {
            unsafe {
                let mut i = Integer { elem: 0 };
                let elem_p: *mut i32 = &mut i.elem;
                *elem_p = *(src as *const i32);
                Ok(Box::new(i))
            }
        }
        _ => Err(format!("Unknown type '{}'", type_name))
    }
}

pub fn build_ty(type_name: &str, row: &str) -> Result<Box<Ty>, String> {
    match type_name {
        "integer" => {
            let elem = row.parse::<i32>().unwrap();
            Ok(Box::new(Integer { elem: elem }))
        }
        _ => Err(format!("Unknown type '{}'", type_name))
    }
}

// Signed 4 bytes integer
pub struct Integer {
    pub elem: i32,
}

impl Ty for Integer {
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
