use tuple::{TupleTableSlot};
use executor::node_seqscan::{ScanState};
use executor::plan_node::PlanNode;

pub struct CountState<'a> {
    lefttree: ScanState<'a>,
    pub result: u64,
}

impl<'a> CountState<'a> {
    pub fn new(lefttree: ScanState<'a>) -> CountState<'a> {
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
