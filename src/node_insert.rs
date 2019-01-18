use tuple::{TupleTableSlot};
use buffer_manager::{BufferManager, RelFileNode};

pub struct InsertState<'a> {
    slot: &'a TupleTableSlot,
    ss_currentRelation: &'a RelFileNode,
}

impl<'a> InsertState<'a> {
    pub fn new(rnode: &'a RelFileNode, slot: &'a TupleTableSlot) -> InsertState<'a> {
        InsertState {
            ss_currentRelation: rnode,
            slot: slot,
        }
    }

    pub fn exec_insert(&mut self, bufmrg: &mut BufferManager) {
        let buf = bufmrg.read_buffer(self.ss_currentRelation.clone(), 0);
        let page = bufmrg.get_page_mut(buf);
        page.add_tuple_slot_entry(self.slot).unwrap();
    }
}