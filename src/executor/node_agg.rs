use tuple::{TupleTableSlot};
use executor::plan_node::PlanNode;

pub struct CountState<'a> {
    lefttree: &'a mut PlanNode,
    pub result: u64,
}

impl<'a> CountState<'a> {
    pub fn new(lefttree: &'a mut PlanNode) -> CountState<'a> {
        CountState {
            lefttree: lefttree,
            result: 0,
        }
    }
}

impl<'a> PlanNode for CountState<'a> {
    // See: `ExecAgg` in pg.
    fn exec(&mut self) -> Option<&TupleTableSlot> {
        loop {
            let opt = self.lefttree.exec();

            match opt {
                Some(_slot) => {
                    self.result = self.result + 1;
                },
                None => break
            }
        }

        None
    }
}
