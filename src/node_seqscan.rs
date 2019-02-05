#![allow(non_snake_case)]
use std::cell::RefCell;

use catalog::catalog::RecordManeger;
use catalog::mini_attribute::MiniAttributeRecord;
use buffer_manager::{Buffer, BlockNumber, BufferManager, InvalidBlockNumber};
use tuple::{TupleTableSlot, HeapTupleData, ItemPointerData};
use off::{FirstOffsetNumber};
use storage_manager::{RelationData};
use ast::Expr;

struct ExprEvaluator<'a> {
    currentTuple: &'a TupleTableSlot,
    expr: &'a Expr,
}

struct PlanState {

}

pub struct ScanState<'a> {
    ps: PlanState,
    // relation being scanned
    ss_currentRelation: &'a RefCell<RelationData>,
    // current scan descriptor for scan
    ss_currentScanDesc: HeapScanDescData<'a>,
    // pointer to slot in tuple table holding scan tuple
    ss_ScanTupleSlot: Box<TupleTableSlot>,
    // The field of PlanState in pg.
    qual: &'a Option<Box<Expr>>,
}

#[derive(Debug)]
struct HeapScanDescData<'a> {
    // heap relation descriptor
    rs_rd: &'a RefCell<RelationData>,

    // total number of blocks in rel
    rs_nblocks: BlockNumber,
    // block # to start at
    rs_startblock: BlockNumber,
    // max number of blocks to scan
    rs_numblocks: BlockNumber,

    // false = scan not init'd yet
    rs_inited: bool,
    // true = scan finished
    rs_finished: bool,
    // current tuple in scan, if any
    rs_ctup: Box<HeapTupleData>,
    // current block # in scan, if any
    rs_cblock: BlockNumber,
    // current buffer in scan, if any
    rs_cbuf: Buffer,
}

impl<'a> ExprEvaluator<'a> {
    fn new(
        currentTuple: &'a TupleTableSlot,
        expr: &'a Expr
    ) -> ExprEvaluator<'a> {
        ExprEvaluator {
            currentTuple: currentTuple,
            expr: expr
        }
    }

    fn eval(&self) -> bool {
        match self.expr {
            Expr::Bool(b) => {
                return b.clone();
            },
            Expr::All => {
                panic!("Unknown expr ({:?})", self.expr);
            },
            Expr::Count => {
                panic!("Unknown expr ({:?})", self.expr);
            }
        }
    }
}

impl<'a> ScanState<'a> {
    // `initscan` in pg.
    pub fn new(
        relation: &'a RefCell<RelationData>,
        rm: &RecordManeger<MiniAttributeRecord>,
        bufmrg: &mut BufferManager,
        qual: &'a Option<Box<Expr>>
    ) -> ScanState<'a> {
        let rnode = &relation.borrow().smgr_rnode;
        let attrs = rm.attributes_clone(rnode.db_oid, rnode.table_oid);
        let attrs_len = attrs.iter().fold(0, |acc, attr| acc + attr.len) as u32;
        let mut tuple = HeapTupleData::new(attrs_len);
        ::tuple::item_pointer_set_invalid(&mut tuple.t_self);
        let rs_nblocks = bufmrg.relation_get_number_of_blocks(&relation.borrow());

        let scan_desc = HeapScanDescData {
            rs_rd: relation,
            rs_nblocks: rs_nblocks,
            rs_startblock: 0,
            rs_numblocks: InvalidBlockNumber,
            rs_inited: false,
            rs_finished: false,
            rs_ctup: Box::new(tuple),
            rs_cblock: InvalidBlockNumber,
            rs_cbuf: Buffer::InvalidBuffer,
        };
        let plan_state = PlanState {};
        let slot = TupleTableSlot::new(attrs);

        ScanState {
            ps: plan_state,
            ss_currentRelation: relation,
            ss_currentScanDesc: scan_desc,
            ss_ScanTupleSlot: Box::new(slot),
            qual: qual,
        }
    }

    // ExecScan in pg.
    pub fn exec_scan(&mut self, bufmrg: &mut BufferManager) -> Option<&TupleTableSlot> {
        loop {
            self.seq_next(bufmrg);

            if self.ss_currentScanDesc.rs_finished {
                return None;
            }

            if self.exec_qual() {
                return Some(self.ss_ScanTupleSlot.as_ref());
            }

            // next tuple
        }
    }

    // SeqNext in pg.
    fn seq_next(&mut self, bufmrg: &mut BufferManager) {
        self.heap_getnext(bufmrg);

        if !self.ss_currentScanDesc.rs_finished {
            let tuple = &self.ss_currentScanDesc.rs_ctup;
            self.ss_ScanTupleSlot.load_data_without_len(tuple.data_ptr(), tuple.t_self.clone());
        }
    }

    // heap_getnext in pg.
    //
    // Get next tuple
    fn heap_getnext(&mut self, bufmrg: &mut BufferManager) {
        self.heapgettup(bufmrg);
    }

    // heapgettup in pg.
    fn heapgettup(&mut self, bufmrg: &mut BufferManager) {
        let scan_desc = &mut self.ss_currentScanDesc;

        let mut lineoff = if !scan_desc.rs_inited {
            let page = scan_desc.rs_startblock;
            let buf = bufmrg.read_buffer(&scan_desc.rs_rd.borrow(), page);
            scan_desc.rs_cbuf = buf;
            scan_desc.rs_cblock = page;
            scan_desc.rs_inited = true;
            FirstOffsetNumber
        } else {
            scan_desc.rs_ctup.get_item_offset_number() + 1
        };

        let lines = bufmrg.get_page(scan_desc.rs_cbuf).page_get_max_offset_number();
        let mut linesleft = lines - lineoff;

        loop {
            // Iterate a page.
            while linesleft > 0 {
                debug!("linesleft: {}", linesleft);

                let dp = bufmrg.get_page(scan_desc.rs_cbuf);
                let mut t_self = ItemPointerData::new();
                ::tuple::item_pointer_set(&mut t_self, scan_desc.rs_cblock, lineoff);
                debug!("lp_len {}", dp.get_item_ref(lineoff).lp_len());
                scan_desc.rs_ctup.load_without_len(dp.get_entry_pointer(lineoff).unwrap(), t_self);

                // Skip deleted record

                if scan_desc.rs_ctup.t_data.heap_keys_updated_p() {
                    debug!("Skip deleted tuple {}", lineoff);
                    lineoff = lineoff + 1;
                    linesleft = linesleft - 1;
                    // next
                } else {
                    debug!("Return tuple {}", lineoff);
                    return
                }
            }

            // if we get here, it means we've exhausted the items on this page and
            // it's time to move to the next.

            if scan_desc.rs_cblock + 1 >= scan_desc.rs_nblocks {
                scan_desc.rs_finished = true;
                return
            }

            // In pg, heapgetpage update `rs_cbuf`, `rs_cblock`
            {
                let page = scan_desc.rs_cblock + 1;
                scan_desc.rs_cblock = page;
                let buf = bufmrg.read_buffer(&scan_desc.rs_rd.borrow(), page);
                scan_desc.rs_cbuf = buf;
                let dp = bufmrg.get_page(scan_desc.rs_cbuf);
                lineoff = FirstOffsetNumber;
                let lines = dp.page_get_max_offset_number();
                linesleft = lines - lineoff;

                debug!("Next page is loaded: (BlockNum {})", page);
            }
        }
    }

    // ExecQual in pg.
    //
    // If we need to current tuple to check qual,
    // use `self.ss_ScanTupleSlot.as_ref()`.
    fn exec_qual(&self) -> bool {
        if self.qual.is_none() {
            // Always condition is met
            return true;
        }

        let evaluator = ExprEvaluator::new(self.ss_ScanTupleSlot.as_ref(), self.qual.as_ref().unwrap().as_ref());
        evaluator.eval()
    }
}
