// off.h in pg.
// this is a 0-based index into the linp (ItemIdData) array in the
// header of each disk page.

pub type OffsetNumber = u16;
pub const FirstOffsetNumber: OffsetNumber = 0;
