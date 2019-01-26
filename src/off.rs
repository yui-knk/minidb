// off.h in pg.
// this is a 0-based index into the linp (ItemIdData) array in the
// header of each disk page.

#![allow(non_upper_case_globals)]
pub type OffsetNumber = u16;
// TODO: FirstOffsetNumber in pg is 1
pub const FirstOffsetNumber: OffsetNumber = 0;
// TODO: InvalidOffsetNumber in pg is 0
pub const InvalidOffsetNumber: OffsetNumber = 0xFFFF;
