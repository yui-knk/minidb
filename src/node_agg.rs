use node_seqscan::{ScanState};
use buffer_manager::{BufferManager};

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

    // `ExecAgg` in pg.
    pub fn exec_agg(&mut self, bufmrg: &mut BufferManager) {
        loop {
            let opt = self.lefttree.exec_scan(bufmrg);

            match opt {
                Some(_slot) => {
                    self.result = self.result + 1;
                },
                None => break
            }
        }
    }
}