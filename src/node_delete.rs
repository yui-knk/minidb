#![allow(non_snake_case)]
use std::cell::RefCell;

use node_seqscan::{ScanState};
use buffer_manager::{BufferManager};
use storage_manager::{RelationData};

pub struct DeleteState<'a> {
    // TODO: Extract this currentRelation to EState
    currentRelation: &'a RefCell<RelationData>,
    lefttree: ScanState<'a>,
    pub count: u64,
}

impl<'a> DeleteState<'a> {
    pub fn new(relation: &'a RefCell<RelationData>, lefttree: ScanState<'a>) -> DeleteState<'a> {
        DeleteState {
            currentRelation: relation,
            lefttree: lefttree,
            count: 0,
        }
    }

    // `ExecDelete` in pg.
    pub fn exec_delete(&mut self, bufmrg: &mut BufferManager) {
        loop {
            let opt = self.lefttree.exec_scan(bufmrg);

            match opt {
                Some(slot) => {
                    let relation = self.currentRelation.borrow();
                    let tid = slot.tid();
                    bufmrg.heap_delete(&relation, tid);
                    self.count = self.count + 1;
                },
                None => break
            }
        }
    }
}
