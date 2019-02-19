#![allow(non_snake_case)]
use std::cell::RefCell;
use std::sync::RwLock;

use tuple::{TupleTableSlot};
use buffer_manager::{BufferManager};
use storage_manager::{RelationData};

pub struct InsertState<'a> {
    slot: &'a TupleTableSlot,
    ss_currentRelation: &'a RefCell<RelationData>,
    bufmrg: &'a RwLock<BufferManager>,
}

impl<'a> InsertState<'a> {
    pub fn new(
        relation: &'a RefCell<RelationData>,
        slot: &'a TupleTableSlot,
        bufmrg: &'a RwLock<BufferManager>
    ) -> InsertState<'a> {
        InsertState {
            ss_currentRelation: relation,
            slot: slot,
            bufmrg: bufmrg,
        }
    }

    // `ExecInsert` in pg.
    pub fn exec_insert(&mut self) {
        self.bufmrg.write().unwrap().heap_insert(&mut self.ss_currentRelation.borrow_mut(), self.slot);
    }
}
