// off.h in pg.
// this is a 0-based index into the linp (ItemIdData) array in the
// header of each disk page.

#![allow(non_upper_case_globals)]
pub type OffsetNumber = u16;
// TODO: FirstOffsetNumber in pg is 1
pub const FirstOffsetNumber: OffsetNumber = 0;
// TODO: InvalidOffsetNumber in pg is 0
pub const InvalidOffsetNumber: OffsetNumber = 0xFFFF;

// In pg, getter fuction is implemented as below,
// which treats 1 as first address of pd_linp array.
//
// ```c
// /*
//  * PageGetItemId
//  *      Returns an item identifier of a page.
//  */
// #define PageGetItemId(page, offsetNumber) \
//     ((ItemId) (&((PageHeader) (page))->pd_linp[(offsetNumber) - 1]))
// ```
