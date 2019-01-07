// Column type

use std::io::{Write};
use byteorder::{BigEndian, WriteBytesExt};

pub trait Ty {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()>;
    fn len(&self) -> usize;
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
    elem: i32,
}

impl Ty for Integer {
    fn write_bytes(&self, wrt: &mut Write) -> std::io::Result<()> {
        wrt.write_i32::<BigEndian>(self.elem)
   }

    fn len(&self) -> usize {
        4
    }
}
