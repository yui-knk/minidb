http://www.interdb.jp/pg/pgsql03.html#_3.1. にあるようにPlannerによって、query treeがplan treeに書き換えられる。その後Executorによってplan treeが実行される。
plan treeは複数のplan nodeから成り立っている。plan nodeには例えばソートを行うnodeSortや、シーケンシャルスキャンを行うnodeSeqscanなどがある。

"src/backend/executor/nodeSort.c"
"src/backend/executor/nodeSeqscan.c"

これらplan nodeの中心となるデータ構造は`XXXState`であり、"src/include/nodes/execnodes.h"で定義されている。

```c
/* ----------------
 *   SortState information
 * ----------------
 */
typedef struct SortState
{
  ScanState ss;       /* its first field is NodeTag */
  bool    randomAccess; /* need random access to sort output? */
  bool    bounded;    /* is the result set bounded? */
  int64   bound;      /* if bounded, how many tuples are needed */
  bool    sort_Done;    /* sort completed yet? */
  bool    bounded_Done; /* value of bounded we did the sort with */
  int64   bound_Done;   /* value of bound we did the sort with */
  void     *tuplesortstate; /* private state of tuplesort.c */
  bool    am_worker;    /* are we a worker? */
  SharedSortInfo *shared_info;  /* one entry per worker */
} SortState;


/* ----------------
 *   ScanState information
 *
 *    ScanState extends PlanState for node types that represent
 *    scans of an underlying relation.  It can also be used for nodes
 *    that scan the output of an underlying plan node --- in that case,
 *    only ScanTupleSlot is actually useful, and it refers to the tuple
 *    retrieved from the subplan.
 *
 *    currentRelation    relation being scanned (NULL if none)
 *    currentScanDesc    current scan descriptor for scan (NULL if none)
 *    ScanTupleSlot    pointer to slot in tuple table holding scan tuple
 * ----------------
 */
typedef struct ScanState
{
  PlanState ps;       /* its first field is NodeTag */
  Relation  ss_currentRelation;
  HeapScanDesc ss_currentScanDesc;
  TupleTableSlot *ss_ScanTupleSlot;
} ScanState;

/* ----------------
 *   SeqScanState information
 * ----------------
 */
typedef struct SeqScanState
{
  ScanState ss;       /* its first field is NodeTag */
  Size    pscan_len;    /* size of parallel heap scan descriptor */
} SeqScanState;
```

これらのState構造体はみな共通して`PlanState`という構造を先頭にもつ。

```c
/* ----------------
 *   ExecProcNodeMtd
 *
 * This is the method called by ExecProcNode to return the next tuple
 * from an executor node.  It returns NULL, or an empty TupleTableSlot,
 * if no more tuples are available.
 * ----------------
 */
typedef TupleTableSlot *(*ExecProcNodeMtd) (struct PlanState *pstate);

/* ----------------
 *    PlanState node
 *
 * We never actually instantiate any PlanState nodes; this is just the common
 * abstract superclass for all PlanState-type nodes.
 * ----------------
 */
typedef struct PlanState
{
  NodeTag   type;

  Plan     *plan;     /* associated Plan node */

  EState     *state;      /* at execution time, states of individual
                 * nodes point to one EState for the whole
                 * top-level plan */

  ExecProcNodeMtd ExecProcNode; /* function to return next tuple */
  ExecProcNodeMtd ExecProcNodeReal; /* actual function, if above is a
                     * wrapper */

  Instrumentation *instrument;  /* Optional runtime stats for this node */
  WorkerInstrumentation *worker_instrument; /* per-worker instrumentation */

  /* Per-worker JIT instrumentation */
  struct SharedJitInstrumentation *worker_jit_instrument;

  /*
   * Common structural data for all Plan types.  These links to subsidiary
   * state trees parallel links in the associated plan tree (except for the
   * subPlan list, which does not exist in the plan tree).
   */
  ExprState  *qual;     /* boolean qual condition */
  struct PlanState *lefttree; /* input plan tree(s) */
  struct PlanState *righttree;

  List     *initPlan;   /* Init SubPlanState nodes (un-correlated expr
                 * subselects) */
  List     *subPlan;    /* SubPlanState nodes in my expressions */

  /*
   * State for management of parameter-change-driven rescanning
   */
  Bitmapset  *chgParam;   /* set of IDs of changed Params */

  /*
   * Other run-time state needed by most if not all node types.
   */
  TupleTableSlot *ps_ResultTupleSlot; /* slot for my result tuples */
  ExprContext *ps_ExprContext;  /* node's expression-evaluation context */
  ProjectionInfo *ps_ProjInfo;  /* info for doing tuple projection */

  /*
   * Scanslot's descriptor if known. This is a bit of a hack, but otherwise
   * it's hard for expression compilation to optimize based on the
   * descriptor, without encoding knowledge about all executor nodes.
   */
  TupleDesc scandesc;
} PlanState;
```

データ構造を初期化するための関数は`ExecInitXXX`である。

```c
SortState *
ExecInitSort(Sort *node, EState *estate, int eflags)
```

```c
SeqScanState *
ExecInitSeqScan(SeqScan *node, EState *estate, int eflags)
```

これらStateの初期化/消去関数は"src/backend/executor/execProcnode.c"の`ExecInitNode`と`ExecEndNode`で適切な関数が呼び出されるようになっている。Stateごとに異なる関数が設定されている`ExecProcNode`はtupleを返すことが期待されており、"src/backend/executor/execMain.c"の`ExecutePlan`などで呼び出される。戻り値がnullのときはそれ以上処理するtupleがないということになる。

例えばnodeSortの`ExecProcNode`は`ExecSort`になっている。Sortは1 tupleずつの処理ができないので、`ExecSort`が呼ばれると自分のsourceとなるNodeから全てのtupleを一度に読み込みsortを行う。一度sortを行うと`sort_Done`が`true`になる。

```c
static TupleTableSlot *
ExecSort(PlanState *pstate)
{
...
    tuplesortstate = tuplesort_begin_heap(tupDesc,
                        plannode->numCols,
                        plannode->sortColIdx,
                        plannode->sortOperators,
                        plannode->collations,
                        plannode->nullsFirst,
                        work_mem,
                        NULL, node->randomAccess);

...

  if (!node->sort_Done)
  {
    ...
    /*
     * Scan the subplan and feed all the tuples to tuplesort.
     */

    for (;;)
    {
      slot = ExecProcNode(outerNode);

      if (TupIsNull(slot))
        break;

      tuplesort_puttupleslot(tuplesortstate, slot);
    }
    ...
    /*
     * finally set the sorted flag to true
     */
    node->sort_Done = true;


    /*
     * Complete the sort.
     */
    tuplesort_performsort(tuplesortstate);
```

一方でnodeSeqscanの場合、`ExecProcNode`は`ExecSeqScan`になっている。`ExecSeqScan`をみるまえに、SeqScan時のメインのデータ構造である`HeapScanDescData`をみておく。Scan対象のRelation(`rs_rd `)、Relationのブロック総数(`rs_nblocks `)、現在のタプル(`rs_ctup`)、現在のブロック(`rs_cblock`)、現在のBuffer(`rs_cbuf`)といった情報を保持している。これが`ScanState`の`ss_currentScanDesc`に保有される。なお`ss_currentScanDesc`は`SeqNext`の初回呼び出し時に`heap_beginscan`で生成されてセットされる。`SeqNext`でタプルを取得するのは、`heap_getnext`とそこから呼ばれる`heapgettup`で行われる。現在のタプルのpage内での位置などは`rs_ctup`から計算する。

```c
/* struct definitions appear in relscan.h */
typedef struct HeapScanDescData *HeapScanDesc;

typedef struct HeapScanDescData
{
  /* scan parameters */
  Relation  rs_rd;      /* heap relation descriptor */
  Snapshot  rs_snapshot;  /* snapshot to see */
  int     rs_nkeys;   /* number of scan keys */
  ScanKey   rs_key;     /* array of scan key descriptors */
  bool    rs_bitmapscan;  /* true if this is really a bitmap scan */
  bool    rs_samplescan;  /* true if this is really a sample scan */
  bool    rs_pageatatime; /* verify visibility page-at-a-time? */
  bool    rs_allow_strat; /* allow or disallow use of access strategy */
  bool    rs_allow_sync;  /* allow or disallow use of syncscan */
  bool    rs_temp_snap; /* unregister snapshot at scan end? */

  /* state set up at initscan time */
  BlockNumber rs_nblocks;   /* total number of blocks in rel */
  BlockNumber rs_startblock;  /* block # to start at */
  BlockNumber rs_numblocks; /* max number of blocks to scan */
  /* rs_numblocks is usually InvalidBlockNumber, meaning "scan whole rel" */
  BufferAccessStrategy rs_strategy; /* access strategy for reads */
  bool    rs_syncscan;  /* report location to syncscan logic? */

  /* scan current state */
  bool    rs_inited;    /* false = scan not init'd yet */
  HeapTupleData rs_ctup;    /* current tuple in scan, if any */
  BlockNumber rs_cblock;    /* current block # in scan, if any */
  Buffer    rs_cbuf;    /* current buffer in scan, if any */
  /* NB: if rs_cbuf is not InvalidBuffer, we hold a pin on that buffer */
  ParallelHeapScanDesc rs_parallel; /* parallel scan information */

  /* these fields only used in page-at-a-time mode and for bitmap scans */
  int     rs_cindex;    /* current tuple's index in vistuples */
  int     rs_ntuples;   /* number of visible tuples on page */
  OffsetNumber rs_vistuples[MaxHeapTuplesPerPage];  /* their offsets */
}     HeapScanDescData;
```

`ExecSeqScan`の実際の処理は`ExecScan`である。`ExecScan`では`SeqNext`でtupleをfetch(`ExecScanFetch`)し、もし条件が指定されていれば`ExecQual`でチェックし条件に合致するときにはそのtupleを返す。条件が指定されてなければ、なにもせずにtupleを返す。という処理になっている。

SortやSeqScanに必要な資源(メモリや一時ファイル)はそれぞれのplan nodeで管理するようになっている。例えばnodeSortの場合、sortする量がすくないときはメモリで、おおいときは一時ファイルを使ってソートを行うが、この管理は`SortState`の`void* tuplesortstate`(実際の型は`Tuplesortstate`)で行なっている。例えばsort量の領域にtupleをinsertする`puttuple_common`関数では`memtuples`や`memtupcount`を使いつつ、場合によっては一時ファイル(ここではtapeと呼ばれている)を使うようにスイッチしている様子がわかる。

```c
/*
 * Shared code for tuple and datum cases.
 */
static void
puttuple_common(Tuplesortstate *state, SortTuple *tuple)
{
  Assert(!LEADER(state));

  switch (state->status)
  {
    case TSS_INITIAL:

      /*
       * Save the tuple into the unsorted array.  First, grow the array
       * as needed.  Note that we try to grow the array when there is
       * still one free slot remaining --- if we fail, there'll still be
       * room to store the incoming tuple, and then we'll switch to
       * tape-based operation.
       */
      if (state->memtupcount >= state->memtupsize - 1)
      {
        (void) grow_memtuples(state);
        Assert(state->memtupcount < state->memtupsize);
      }
      state->memtuples[state->memtupcount++] = *tuple;
...
      /*
       * Done if we still fit in available memory and have array slots.
       */
      if (state->memtupcount < state->memtupsize && !LACKMEM(state))
        return;

      /*
       * Nope; time to switch to tape-based operation.
       */
      inittapes(state, true);
```

# PlanState

`PlanState`のメンバーのうち、`lefttree`がそのPlanに対する入力となる。

```c
  struct PlanState *lefttree; /* input plan tree(s) */
  struct PlanState *righttree;
```

# count 機能を追加する

以下の実行結果から`Aggregate`がCountの処理を担っているとわかる。

```
lusiadas=# explain select * from films;
                        QUERY PLAN
----------------------------------------------------------
 Seq Scan on films  (cost=0.00..13.80 rows=380 width=184)
(1 row)

lusiadas=# explain select count(1) from films;
                          QUERY PLAN
--------------------------------------------------------------
 Aggregate  (cost=14.75..14.76 rows=1 width=8)
   ->  Seq Scan on films  (cost=0.00..13.80 rows=380 width=0)
(2 rows)
```

メインとなる処理は"nodeAgg.c"に記述されている。初期化/消去子は"execProcnode.c"の`ExecInitNode`/`ExecEndNode`より、それぞれ`ExecInitAgg`/`ExecEndAgg`であるとわかる。また`aggstate->ss.ps.ExecProcNode = ExecAgg;`とあることから、各tupleに対する処理は`ExecAgg`で行うとわかる。
`agg_retrieve_direct`のなかでは`fetch_input_tuple`を呼び出してtupleを取得する。

```c
outerslot = fetch_input_tuple(aggstate);
```

`fetch_input_tuple`は以下のようになっており、`lefttree`の`ExecProcNode`を呼び出している。

```c
slot = ExecProcNode(outerPlanState(aggstate));
```

Aggregatorは入力となるtupleを一度に全部イテレーターするので`agg_retrieve_direct`ではloopして`fetch_input_tuple`する。

```c
  while (!aggstate->agg_done)
  {
      ...
          outerslot = fetch_input_tuple(aggstate);
          if (TupIsNull(outerslot))
          {
            /* no more outer-plan tuples available */
            if (hasGroupingSets)
            {
              aggstate->input_done = true;
              break;
            }
            else
            {
              aggstate->agg_done = true;
              break;
            }
          }
```

# レコード削除を実装する

Ref: http://www.interdb.jp/pg/pgsql05.html#_5.3.

```
lusiadas=# explain delete from films;
                          QUERY PLAN
--------------------------------------------------------------
 Delete on films  (cost=0.00..13.80 rows=380 width=6)
   ->  Seq Scan on films  (cost=0.00..13.80 rows=380 width=6)
(2 rows)
```

Delete処理は`ExecDelete`関数("nodeModifyTable.c")によって行われる。この関数のメインの処理は`heap_delete`("heapam.c")である。この関数では

(1) tidからblock番号を取得する
(2) block番号とRelationを指定して、bufferにデータを読み込む
(3) pageから当該tupleのデータをtuple構造体に読み込む
(4) t_infomask2のHEAP_KEYS_UPDATEDをたてる(new_infomask2を参照)
(5) HeapTupleHeaderSetXmaxしてt_xmaxをセットする


```c
  block = ItemPointerGetBlockNumber(tid);
  buffer = ReadBuffer(relation, block);
  page = BufferGetPage(buffer);
...
  lp = PageGetItemId(page, ItemPointerGetOffsetNumber(tid));
  Assert(ItemIdIsNormal(lp));

  tp.t_tableOid = RelationGetRelid(relation);
  tp.t_data = (HeapTupleHeader) PageGetItem(page, lp);
  tp.t_len = ItemIdGetLength(lp);
  tp.t_self = *tid;

...

  compute_new_xmax_infomask(HeapTupleHeaderGetRawXmax(tp.t_data),
                tp.t_data->t_infomask, tp.t_data->t_infomask2,
                xid, LockTupleExclusive, true,
                &new_xmax, &new_infomask, &new_infomask2);

...

  /* store transaction information of xact deleting the tuple */
  tp.t_data->t_infomask &= ~(HEAP_XMAX_BITS | HEAP_MOVED);
  tp.t_data->t_infomask2 &= ~HEAP_KEYS_UPDATED;
  tp.t_data->t_infomask |= new_infomask;
  tp.t_data->t_infomask2 |= new_infomask2;
  HeapTupleHeaderClearHotUpdated(tp.t_data);
  HeapTupleHeaderSetXmax(tp.t_data, new_xmax);
  HeapTupleHeaderSetCmax(tp.t_data, cid, iscombo);
```
