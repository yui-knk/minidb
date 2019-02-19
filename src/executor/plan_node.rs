use tuple::{TupleTableSlot};

// typedef struct Plan in pg.
pub trait PlanNode {
    fn exec(&mut self) -> Option<&TupleTableSlot>;
}
