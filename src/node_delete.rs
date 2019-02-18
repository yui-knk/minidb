#![allow(non_snake_case)]
use std::cell::RefCell;
use std::sync::RwLock;

use node_seqscan::{ScanState};
use buffer_manager::{BufferManager};
use storage_manager::{RelationData};

pub struct DeleteState<'a> {
    // TODO: Extract this currentRelation to EState
    currentRelation: &'a RefCell<RelationData>,
    lefttree: ScanState<'a>,
    pub count: u64,
    bufmrg: &'a RwLock<BufferManager>,
}

impl<'a> DeleteState<'a> {
    pub fn new(
        relation: &'a RefCell<RelationData>,
        lefttree: ScanState<'a>,
        bufmrg: &'a RwLock<BufferManager>,
    ) -> DeleteState<'a> {
        DeleteState {
            currentRelation: relation,
            lefttree: lefttree,
            count: 0,
            bufmrg: bufmrg,
        }
    }

    // `ExecDelete` in pg.
    pub fn exec_delete(&mut self) {
        loop {
            let opt = self.lefttree.exec_scan();

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
    }
}
