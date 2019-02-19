#![allow(non_snake_case)]
use std::cell::RefCell;
use std::sync::RwLock;

use tuple::{TupleTableSlot};
use buffer_manager::{BufferManager};
use storage_manager::{RelationData};
use executor::plan_node::{PlanNode};

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
}

impl<'a> PlanNode for InsertState<'a> {
    // `ExecInsert` in pg.
    fn exec(&mut self) -> Option<&TupleTableSlot> {
        self.bufmrg.write().unwrap().heap_insert(&mut self.ss_currentRelation.borrow_mut(), self.slot);
        Some(self.slot)
    }
}
