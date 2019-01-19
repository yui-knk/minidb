#![allow(non_snake_case)]

use catalog::catalog::RecordManeger;
use catalog::mini_attribute::MiniAttributeRecord;
use buffer_manager::{Buffer, BlockNum, RelFileNode, BufferManager};
use tuple::{TupleTableSlot, HeapTupleData, ItemPointerData};
use off::{FirstOffsetNumber};

struct PlanState {

}

pub struct ScanState<'a> {
    ps: PlanState,
    // relation being scanned
    // ss_currentRelation: &'a Relation,
    ss_currentRelation: &'a RelFileNode,
    // current scan descriptor for scan
    ss_currentScanDesc: HeapScanDescData<'a>,
    // pointer to slot in tuple table holding scan tuple
    ss_ScanTupleSlot: Box<TupleTableSlot>,
}

struct HeapScanDescData<'a> {
    // heap relation descriptor
    // rs_rd: &'a Relation,
    rs_rd: &'a RelFileNode,

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
    // 
    pub fn new(rnode: &'a RelFileNode, rm: &'a RecordManeger<MiniAttributeRecord>) -> ScanState<'a> {
        let attrs = rm.attributes_clone(rnode.db_oid, rnode.table_oid);
        let attrs_len = attrs.iter().fold(0, |acc, attr| acc + attr.len) as u32;
        let tuple = HeapTupleData::new(attrs_len);

        let scan_desc = HeapScanDescData {
            rs_rd: rnode,
            rs_nblocks: 1,
            rs_startblock: 1,
            rs_numblocks: 1,
            rs_inited: false,
            rs_finished: false,
            rs_ctup: Box::new(tuple),
            rs_cblock: 0,
            rs_cbuf: Buffer::InvalidBuffer,
        };
        let plan_state = PlanState {};
        let slot = TupleTableSlot::new(attrs);

        ScanState {
            ps: plan_state,
            ss_currentRelation: rnode,
            ss_currentScanDesc: scan_desc,
            ss_ScanTupleSlot: Box::new(slot),
        }
    }

    // ExecScan in pg.
    pub fn exec_scan(&mut self, bufmrg: &mut BufferManager) {
        loop {
            let opt = self.seq_next(bufmrg);

            match opt {
                Some(slot) => {
                    for i in 0..(slot.attrs_count()) {
                        let ty = slot.get_column(i);
                        println!("{:?}", ty.as_string());
                    }
                },
                None => break
            }
        }
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

    // Get next tuple
    // heap_getnext in pg.
    fn heap_getnext(&mut self, bufmrg: &mut BufferManager) -> bool {
        self.heapgettup(bufmrg);
        let scan_desc = &self.ss_currentScanDesc;
        !scan_desc.rs_finished
    }

    fn heapgettup(&mut self, bufmrg: &mut BufferManager) {
        let scan_desc = &mut self.ss_currentScanDesc;
        let mut lineoff = FirstOffsetNumber;
        let mut linesleft = 0;

        if !scan_desc.rs_inited {
            let page = scan_desc.rs_startblock;
            let buf = bufmrg.read_buffer((*scan_desc.rs_rd).clone(), page);
            scan_desc.rs_cbuf = buf;
            scan_desc.rs_cblock = page;
            lineoff = FirstOffsetNumber;

            scan_desc.rs_inited = true;
        } else {
            lineoff = scan_desc.rs_ctup.get_item_offset_number() + 1;
        }

        // heapgettup in pg.
        let dp = bufmrg.get_page(scan_desc.rs_cbuf);
        let lines = dp.page_get_max_offset_number();
        linesleft = lines - lineoff;

        loop {
            // Iterate a page.
            while linesleft > 0 {
                println!("linesleft: {:?}", linesleft);
                let mut t_self = ItemPointerData::new();
                t_self.ip_blkid = dp.get_item(lineoff);
                t_self.ip_posid = lineoff;

                scan_desc.rs_ctup.load_without_len(dp.get_entry_pointer(lineoff).unwrap(), t_self);
                linesleft = linesleft - 1;
                return
            }

            scan_desc.rs_finished = true;
            return
        }
    }
}
