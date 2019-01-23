use std::cell::RefCell;

use tuple::{TupleTableSlot};
use buffer_manager::{BufferManager};
use storage_manager::{RelationData};

pub struct InsertState<'a> {
    slot: &'a TupleTableSlot,
    ss_currentRelation: &'a RefCell<RelationData>,
}

impl<'a> InsertState<'a> {
    pub fn new(relation: &'a RefCell<RelationData>, slot: &'a TupleTableSlot) -> InsertState<'a> {
        InsertState {
            ss_currentRelation: relation,
            slot: slot,
        }
    }

    // `ExecInsert` in pg.
    pub fn exec_insert(&mut self, bufmrg: &mut BufferManager) {
        bufmrg.heap_insert(&mut self.ss_currentRelation.borrow_mut(), self.slot);
    }
}
