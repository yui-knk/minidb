#![allow(non_snake_case)]
use tuple::{TupleTableSlot};
use executor::node_seqscan::{ScanState};
use executor::plan_node::PlanNode;

pub struct SortState<'a> {
    lefttree: ScanState<'a>,
    sort_Done: bool,
    memtuples: Vec<Box<TupleTableSlot>>,
    current: usize, // array index (points current tuple index)
    // Most simple version of comparetup
    target_col_name: String,
}

impl<'a> SortState<'a> {
    pub fn new(ss: ScanState<'a>, target_col_name: String) -> SortState {
        SortState {
            lefttree: ss,
            sort_Done: false,
            memtuples: vec![],
            current: 0,
            target_col_name: target_col_name,
        }
    }

    // fn tuplesort_puttupleslot(&mut self, slot: Box<TupleTableSlot>) {
    //     self.memtuples.push(slot);
    // }

    fn tuplesort_performsort(&mut self) {
        let col_name = self.target_col_name.clone();

        self.memtuples.sort_by(|a, b| {
           let i_a = a.get_index_from_name(&col_name);
           let i_b = b.get_index_from_name(&col_name);
           let col_a = a.get_column(i_a);
           let col_b = b.get_column(i_b);

           col_a.as_string().cmp(&col_b.as_string())
        })
    }
}

impl<'a> PlanNode for SortState<'a> {
    // ExecSort in pg.
    fn exec(&mut self) -> Option<&TupleTableSlot> {
        if !self.sort_Done {
            loop {
                let slot = self.lefttree.exec();

                match slot {
                    Some(s) => {
                        // tuplesort_puttupleslot
                        self.memtuples.push(Box::new((*s).clone()));
                    },
                    None => {
                        break;
                    }
                }
            }

            self.tuplesort_performsort();
            self.sort_Done = true;
        }

        if self.current < self.memtuples.len() {
            let slot = &self.memtuples[self.current];
            self.current = self.current + 1;
            return Some(slot.as_ref());
        }

        return None;
    }
}
