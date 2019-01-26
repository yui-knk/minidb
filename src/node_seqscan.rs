#![allow(non_snake_case)]
use std::cell::RefCell;

use catalog::catalog::RecordManeger;
use catalog::mini_attribute::MiniAttributeRecord;
use buffer_manager::{Buffer, BlockNum, BufferManager, InvalidBlockNumber};
use tuple::{TupleTableSlot, HeapTupleData, ItemPointerData};
use off::{FirstOffsetNumber};
use storage_manager::{RelationData};

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
}

#[derive(Debug)]
struct HeapScanDescData<'a> {
    // heap relation descriptor
    rs_rd: &'a RefCell<RelationData>,

    // total number of blocks in rel
    rs_nblocks: BlockNum,
    // block # to start at
    rs_startblock: BlockNum,
    // max number of blocks to scan
    rs_numblocks: BlockNum,

    // false = scan not init'd yet
    rs_inited: bool,
    // true = scan finished
    rs_finished: bool,
    // current tuple in scan, if any
    rs_ctup: Box<HeapTupleData>,
    // current block # in scan, if any
    rs_cblock: BlockNum,
    // current buffer in scan, if any
    rs_cbuf: Buffer,
}

impl<'a> ScanState<'a> {
    // `initscan` in pg.
    pub fn new(
        relation: &'a RefCell<RelationData>,
        rm: &RecordManeger<MiniAttributeRecord>,
        bufmrg: &mut BufferManager
    ) -> ScanState<'a> {
        let rnode = &relation.borrow().smgr_rnode;
        let attrs = rm.attributes_clone(rnode.db_oid, rnode.table_oid);
        let attrs_len = attrs.iter().fold(0, |acc, attr| acc + attr.len) as u32;
        let tuple = HeapTupleData::new(attrs_len);
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
        }
    }

    // ExecScan in pg.
    pub fn exec_scan(&mut self, bufmrg: &mut BufferManager) -> Option<&TupleTableSlot> {
        self.seq_next(bufmrg)
    }

    // SeqNext in pg.
    fn seq_next(&mut self, bufmrg: &mut BufferManager) -> Option<&TupleTableSlot> {
        let b = self.heap_getnext(bufmrg);

        if b {
            let tuple = &self.ss_currentScanDesc.rs_ctup;
            self.ss_ScanTupleSlot.load_data_without_len(tuple.data_ptr(), tuple.t_self.clone());
            Some(self.ss_ScanTupleSlot.as_ref())
        } else {
            None
        }
    }

    // heap_getnext in pg.
    //
    // Get next tuple
    fn heap_getnext(&mut self, bufmrg: &mut BufferManager) -> bool {
        self.heapgettup(bufmrg);
        let scan_desc = &self.ss_currentScanDesc;
        !scan_desc.rs_finished
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
                println!("linesleft: {:?}", linesleft);
                let dp = bufmrg.get_page(scan_desc.rs_cbuf);
                let mut t_self = ItemPointerData::new();
                t_self.ip_blkid = dp.get_item(lineoff);
                t_self.ip_posid = lineoff;

                scan_desc.rs_ctup.load_without_len(dp.get_entry_pointer(lineoff).unwrap(), t_self);
                return
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
            }
        }
    }
}
