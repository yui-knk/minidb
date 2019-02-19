#![allow(non_snake_case)]
use std::cell::RefCell;
use std::sync::RwLock;

use tuple::{TupleTableSlot};
use buffer_manager::{BufferManager};
use storage_manager::{RelationData};
use executor::plan_node::PlanNode;

pub struct DeleteState<'a> {
    // TODO: Extract this currentRelation to EState
    currentRelation: &'a RefCell<RelationData>,
    lefttree: &'a mut PlanNode,
    pub count: u64,
    bufmrg: &'a RwLock<BufferManager>,
}

impl<'a> DeleteState<'a> {
    pub fn new(
        relation: &'a RefCell<RelationData>,
        lefttree: &'a mut PlanNode,
        bufmrg: &'a RwLock<BufferManager>,
    ) -> DeleteState<'a> {
        DeleteState {
            currentRelation: relation,
            lefttree: lefttree,
            count: 0,
            bufmrg: bufmrg,
        }
    }
}

impl<'a> PlanNode for DeleteState<'a> {
    // `ExecDelete` in pg.
    fn exec(&mut self) -> Option<&TupleTableSlot> {
        loop {
            let opt = self.lefttree.exec();

            match opt {
                Some(slot) => {
                    let relation = self.currentRelation.borrow();
                    let tid = slot.tid();
                    self.bufmrg.write().unwrap().heap_delete(&relation, tid);
                    self.count = self.count + 1;
                },
                None => break
            }
        }

        None
    }
}
