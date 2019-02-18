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

# whereを実装する

文法について確認する("gram.y")。

```
where_clause:
      WHERE a_expr              { $$ = $2; }
      | /*EMPTY*/               { $$ = NULL; }
    ;

a_expr:   c_expr                  { $$ = $1; }
...
      | a_expr '=' a_expr
        { $$ = (Node *) makeSimpleA_Expr(AEXPR_OP, "=", $1, $3, @2); }

c_expr:   columnref               { $$ = $1; }
      | AexprConst              { $$ = $1; }

/*
 * Constants
 */
AexprConst: Iconst
        {
          $$ = makeIntConst($1, @1);
        }
      | FCONST
        {
          $$ = makeFloatConst($1, @1);
        }
```

`makeSimpleA_Expr`では`makeNode(A_Expr)`がよばれるので、`T_A_Expr`というtypeをもったノード(`A_Expr`)がつくられる。
このようにして作られた`where_clause`は`SelectStmt->whereClause`に格納される。この時点では個々の条件は`T_A_Expr`というtypeをもったノード(`A_Expr` "parsenodes.h")である。

```
simple_select:
      SELECT opt_all_clause opt_target_list
      into_clause from_clause where_clause
      group_clause having_clause window_clause
        {
          SelectStmt *n = makeNode(SelectStmt);
          n->targetList = $3;
          n->intoClause = $4;
          n->fromClause = $5;
          n->whereClause = $6;
          n->groupClause = $7;
          n->havingClause = $8;
          n->windowClause = $9;
          $$ = (Node *)n;
        }
```

"gram.y"で生成された木は、次に`pg_analyze_and_rewrite`にかけられる(main.md参照)。
`pg_analyze_and_rewrite` -> `parse_analyze` -> `transformTopLevelStmt` -> `transformOptionalSelectInto` -> `transformStmt` と呼び出しが続き、`transformStmt`ではNodeのtypeによる分岐がはいる。`T_SelectStmt`の場合の`transformSelectStmt`をみていくと、`transformWhereClause` -> `transformWhereClause` -> `transformExpr` -> `transformExprRecurse` と呼び出しが続く。`transformExprRecurse`では式の種類による分類がある。`T_A_Expr`かつ`AEXPR_OP`のケースでは`transformAExprOp`がよばれる。もっとも汎用的なケースでは`make_op`がよばれ、この戻り値は`OpExpr`Node("primnodes.h")となる。
ここで`transformSelectStmt`の戻り値は`Query`("parsenodes.h")となっており、`transformWhereClause`の結果はjoinの情報とともに`FromExpr`にくるまれて、`Query->jointree`に格納される。

```c
/*****************************************************************************
 *  Query Tree
 *****************************************************************************/

/*
 * Query -
 *    Parse analysis turns all statements into a Query tree
 *    for further processing by the rewriter and planner.
```

```c
  qry->jointree = makeFromExpr(pstate->p_joinlist, qual);
```

`pg_analyze_and_rewrite`で処理された木は、次に`pg_plan_queries`にかけられる(main.md参照)。ここでの処理はQuery optimizeがメインとなる。
`pg_plan_queries`のコメントにあるように戻り値は`PlannedStmt`Node("plannodes.h")になる。
処理は`pg_plan_queries` -> `pg_plan_query` -> `planner` -> `standard_planner`と進む。`subquery_planner`はsub queryの処理を再帰的に行うことを想定したentrypointであり、top levelのstmtを処理する`standard_planner`も主な処理は`subquery_planner`で行う。
`subquery_planner`のうち、`where`(事前の書き換えでここでは`Query->jointree`に格納されている)に関連する処理に注目すると、`preprocess_qual_conditions`という関数を呼び出していることがわかる(が、ここでは構造が大きく変わったりはしていなさそう)。
`standard_planner`内部で`subquery_planner`の処理が終わると、その結果は`create_plan`に渡される。`create_plan`では`Plan`つまりPlan nodeが生成される。

```c
  /* primary planning entry point (may recurse for subqueries) */
  root = subquery_planner(glob, parse, NULL,
              false, tuple_fraction);

  /* Select best Path and turn it into a Plan */
  final_rel = fetch_upper_rel(root, UPPERREL_FINAL, NULL);
  best_path = get_cheapest_fractional_path(final_rel, tuple_fraction);

  top_plan = create_plan(root, best_path);
```

`pg_plan_queries`で処理された木は実行に移される(main.md参照)。`Portal`に対する一連の処理の中で`ExecutorStart`が実行される。`ExecutorStart` -> `standard_ExecutorStart` -> `InitPlan` -> `ExecInitNode`と処理が続く。
ここの`ExecInitNode`は`Plan`をもとに`PlanState`("execnodes.h")を作成する。`ExecInitNode`の中ではNodeの種類に応じて`ExecInitXXX`という関数が呼ばれる。SeqScanの場合は`ExecInitSeqScan`がよばれる。この関数の最後で`PlanState->qual`がセットされる。

```c
  /*
   * initialize child expressions
   */
  scanstate->ss.ps.qual =
    ExecInitQual(node->plan.qual, (PlanState *) scanstate);
```

`InitPlan`をみるとわかるとおり、`QueryDesc *queryDesc`が各種構造体のハブになっている。例えば`InitPlan`では`Plan`をもとに`PlanState`を作成するが、これは`queryDesc->plannedstmt->planTree`をもとに作成した`PlanState`を`queryDesc->planstate`に格納する処理となっている(`PlannedStmt`が`struct Plan *planTree;`を所有している)。

`ExecInitQual`の中でexpressionがコンパイルされる様子をみていく。`ExecInitQual`は`ExprState`をアロケートし初期化する。そして`ExecInitExprRec`を呼び出してexpressionをstepに変換して`ExprState`に積んでいく。そして最後に`ExecReadyExpr`を呼び出す。`ExecReadyExpr`は`ExecReadyInterpretedExpr`を呼び出す。
`ExecReadyInterpretedExpr`では`state->evalfunc = ExecInterpExprStillValid;`とセットしたのちに、stepに応じて`ExprState->evalfunc_private`を決定する。ここで最も汎用的な`evalfunc_private`は`ExecInterpExpr`である。
なおここで設定された`evalfunc`は`ExecEvalExpr`もしくは`ExecEvalExprSwitchContext`を使って呼び出すことができる。
たとえばSeqScanの場合、`ExecScan` (`qual = node->ps.qual;`) -> `ExecQual` -> `ExecEvalExprSwitchContext` (`retDatum = state->evalfunc(state, econtext, isNull);`) と呼び出される。

"="は実際にはどのような命令になるのか？
前述したように`pg_analyze_and_rewrite`の処理の中の`make_op`で`T_A_Expr`だったNodeは`OpExpr`Node("primnodes.h")になっている。これは`ExecInitExprRec`の中では`case T_OpExpr:`に該当するため`ExecInitFunc`が呼ばれる。

```c
case T_OpExpr:
  {
    OpExpr     *op = (OpExpr *) node;

    ExecInitFunc(&scratch, node,
           op->args, op->opfuncid, op->inputcollid,
           state);
    ExprEvalPushStep(state, &scratch);
    break;
  }
```

`ExecInitFunc`のなかで`scratch->opcode`に代入されうるのは次の4つ:

* `EEOP_FUNCEXPR_STRICT`
* `EEOP_FUNCEXPR`
* `EEOP_FUNCEXPR_STRICT_FUSAGE`
* `EEOP_FUNCEXPR_FUSAGE`

```c
  /* Insert appropriate opcode depending on strictness and stats level */
  if (pgstat_track_functions <= flinfo->fn_stats)
  {
    if (flinfo->fn_strict && nargs > 0)
      scratch->opcode = EEOP_FUNCEXPR_STRICT;
    else
      scratch->opcode = EEOP_FUNCEXPR;
  }
  else
  {
    if (flinfo->fn_strict && nargs > 0)
      scratch->opcode = EEOP_FUNCEXPR_STRICT_FUSAGE;
    else
      scratch->opcode = EEOP_FUNCEXPR_FUSAGE;
  }
```

たとえば`EEOP_FUNCEXPR`の場合、`ExecInterpExpr`では以下のように`op->d.func.fn_addr`の呼び出しになる。

```c
    /*
     * Function-call implementations. Arguments have previously been
     * evaluated directly into fcinfo->args.
     *
     * As both STRICT checks and function-usage are noticeable performance
     * wise, and function calls are a very hot-path (they also back
     * operators!), it's worth having so many separate opcodes.
     *
     * Note: the reason for using a temporary variable "d", here and in
     * other places, is that some compilers think "*op->resvalue = f();"
     * requires them to evaluate op->resvalue into a register before
     * calling f(), just in case f() is able to modify op->resvalue
     * somehow.  The extra line of code can save a useless register spill
     * and reload across the function call.
     */
    EEO_CASE(EEOP_FUNCEXPR)
    {
      FunctionCallInfo fcinfo = op->d.func.fcinfo_data;
      Datum   d;

      fcinfo->isnull = false;
      d = op->d.func.fn_addr(fcinfo);
      *op->resvalue = d;
      *op->resnull = fcinfo->isnull;

      EEO_NEXT();
    }
```

この`fn_addr`がどこからくるのかというと、それは`ExecInitFunc`である。

```c
  /* Set up the primary fmgr lookup information */
  fmgr_info(funcid, flinfo);
  fmgr_info_set_expr((Node *) node, flinfo);

  /* Initialize function call parameter structure too */
  InitFunctionCallInfoData(*fcinfo, flinfo,
               nargs, inputcollid, NULL, NULL);

  /* Keep extra copies of this info to save an indirection at runtime */
  scratch->d.func.fn_addr = flinfo->fn_addr;
  scratch->d.func.nargs = nargs;
```

もっとも基礎的な組み込み関数の場合、`fmgr_info` -> `fmgr_info_cxt_security` -> `fmgr_isbuiltin`と呼び出され、`fmgr_isbuiltin`の中で関数のアドレスを解決する。関数のアドレスは`fmgr_builtins`に格納されている。`fmgr_builtins`は"src/backend/utils/Gen_fmgrtab.pl"からbuild時に生成される、"src/backend/utils/fmgrtab.c"というファイルで定義されている。

```c
const FmgrBuiltin fmgr_builtins[] = {
  { 31, "byteaout", 1, true, false, byteaout },
  { 33, "charout", 1, true, false, charout },
  { 34, "namein", 1, true, false, namein },
  { 35, "nameout", 1, true, false, nameout },
  { 38, "int2in", 1, true, false, int2in },
...
```

```
  else
  {
    /* otherwise, binary operator */
    ltypeId = exprType(ltree);
    rtypeId = exprType(rtree);
    tup = oper(pstate, opname, ltypeId, rtypeId, false, location);
  }

  opform = (Form_pg_operator) GETSTRUCT(tup);
...
  result->opfuncid = opform->oprcode;

```

# `where id = 10` を実装する

gram.yで作られるNodeは`ColumnRef`である。

```c
where_clause:
      WHERE a_expr              { $$ = $2; }
      | /*EMPTY*/               { $$ = NULL; }
    ;

a_expr:   c_expr                  { $$ = $1; }

c_expr:   columnref               { $$ = $1; }

columnref:  ColId
        {
          $$ = makeColumnRef($1, NIL, @1, yyscanner);
        }
      | ColId indirection
        {
          $$ = makeColumnRef($1, $2, @1, yyscanner);
        }
    ;

static Node *
makeColumnRef(char *colname, List *indirection,
        int location, core_yyscan_t yyscanner)
{
  /*
   * Generate a ColumnRef node, with an A_Indirection node added if there
   * is any subscripting in the specified indirection list.  However,
   * any field selection at the start of the indirection list must be
   * transposed into the "fields" part of the ColumnRef node.
   */
  ColumnRef  *c = makeNode(ColumnRef);
```

`transformExprRecurse`では`transformColumnRef`がよばれる。`ColumnRef`は`table.id`や`db.table.id`などの指定が可能だが、ここでは`id`のケースをみていく。
`transformColumnRef` -> `colNameToVar` -> `scanRTEForColumn` -> `make_var` -> `makeVar`と呼び出しが続き、`makeVar`でVar Node("primnodes.h")が作られる。
`transformColumnRef`の時点では引数は`ColumnRef *cref`である。つまりこの時点ではカラム名の文字列が渡ってくる。`colNameToVar`の時点の引数は`const char *colname`であるので、ここでもカラム名は文字列である。`colNameToVar`のなかでは`pstate->p_namespace`リストをイテレートしながら`scanRTEForColumn`を呼び出していくが、そのとき`nsitem->p_rte`を取り出して`scanRTEForColumn`に渡している。

```c
    foreach(l, pstate->p_namespace)
    {
      ParseNamespaceItem *nsitem = (ParseNamespaceItem *) lfirst(l);
      RangeTblEntry *rte = nsitem->p_rte;
      Node     *newresult;

      /* Ignore table-only items */
      if (!nsitem->p_cols_visible)
        continue;
      /* If not inside LATERAL, ignore lateral-only items */
      if (nsitem->p_lateral_only && !pstate->p_lateral_active)
        continue;

      /* use orig_pstate here to get the right sublevels_up */
      newresult = scanRTEForColumn(orig_pstate, rte, colname, location,
                     0, NULL);
```

`scanRTEForColumn`のなかでは`rte->eref->colnames`を順番にみながら、`const char *colname`と一致するものを探し、Var Nodeをつくる。Var Nodeのレベルでは`colname`ではなく`attrno`という数値になっている。`varno`は`pstate->p_rtable`のリストの何個目の要素に当該カラムが含まれているかを表す。`make_var`のなかで`RTERangeTablePosn`を使って解決している。

```c
typedef struct Var
{
  Expr    xpr;
  Index   varno;      /* index of this var's relation in the range
                 * table, or INNER_VAR/OUTER_VAR/INDEX_VAR */
  AttrNumber  varattno;   /* attribute number of this var, or zero for
                 * all attrs ("whole-row Var") */
  ...
```

T_Var Nodeは`ExecInitExprRec`によって`EEOP_SCAN_VAR`などのstepになる。stepの評価は以下のようになっており、現在のtupleの該当するカラムをindexアクセスする。

```c
    EEO_CASE(EEOP_SCAN_VAR)
    {
      int     attnum = op->d.var.attnum;

      /* See EEOP_INNER_VAR comments */

      Assert(attnum >= 0 && attnum < scanslot->tts_nvalid);
      *op->resvalue = scanslot->tts_values[attnum];
      *op->resnull = scanslot->tts_isnull[attnum];

      EEO_NEXT();
    }
```

ここで`scanslot`は`econtext->ecxt_scantuple`のことである。`ExprContext`は式のevaluationに必要な情報を保持している。`ecxt_scantuple`は現在処理しているtupleを指しており、例えば`ExecScan`のなかでtupleをfetchするごとに代入される。

```c
    slot = ExecScanFetch(node, accessMtd, recheckMtd);
    ...
    /*
     * place the current tuple into the expr context
     */
    econtext->ecxt_scantuple = slot;
```

## ParseState

`ParseState`はparse analysis時のコンテキストを管理するもので、joinのリストやrange tableのリストをもっている。

```c
/*
 * State information used during parse analysis
 *
...
struct ParseState
{
  struct ParseState *parentParseState;  /* stack link */
  const char *p_sourcetext; /* source text, or NULL if not available */
  List     *p_rtable;   /* range table so far */
  List     *p_joinexprs;  /* JoinExprs for RTE_JOIN p_rtable entries */
  List     *p_joinlist;   /* join items so far (will become FromExpr
                 * node's fromlist) */
  List     *p_namespace;  /* currently-referenceable RTEs (List of
                 * ParseNamespaceItem) */
```

`ParseState`は`parse_analyze`のタイミングでつくられる。

```c
Query *
parse_analyze(RawStmt *parseTree, const char *sourceText,
        Oid *paramTypes, int numParams,
        QueryEnvironment *queryEnv)
{
  ParseState *pstate = make_parsestate(NULL);
```

`ParseState`のメンバーのうち、あとのフェーズで`ColumnRef`などのRelation解決に使われるのが`p_namespace`である。`p_namespace`は`transformFromClause`などで拡張される。`transformFromClause`の場合、`transformFromClause` -> `transformFromClauseItem` -> `transformTableEntry` -> `addRangeTableEntry` (ここで戻り値の RangeTblEntry Node を作成) -> `parserOpenTable` -> `heap_openrv_extended` -> `relation_openrv_extended` -> `RangeVarGetRelid` -> `RangeVarGetRelidExtended`とよびだして、`RangeVar`をもとにRelationのOidを解決する。`relation_openrv_extended`でOidからRelationをひき、`addRangeTableEntry`でRangeTblEntry Nodeに必要な情報をRelationからコピーする。直近必要な情報としてカラム列に関する情報(`rte->eref->colnames`)がある。これは`buildRelationAliases`のなかで生成される。

```c
RangeTblEntry *
addRangeTableEntry(ParseState *pstate,
           RangeVar *relation,
           Alias *alias,
           bool inh,
           bool inFromCl)
{
  ...
  /*
   * Build the list of effective column names using user-supplied aliases
   * and/or actual column names.
   */
  rte->eref = makeAlias(refname, NIL);
  buildRelationAliases(rel->rd_att, alias, rte->eref);
  ...
  /*
   * Add completed RTE to pstate's range table list, but not to join list
   * nor namespace --- caller must do that if appropriate.
   */
  pstate->p_rtable = lappend(pstate->p_rtable, rte);
```

また`addRangeTableEntry`では`pstate->p_rtable`に新しく作った`RangeTblEntry`をappendする。

`transformFromClauseItem`ではいま作った`RangeTblEntry`をもとに`ParseNamespaceItem`をつくり、`namespace`の指しているアドレスに代入する。

```c
    /* Check if it's a CTE or tuplestore reference */
    rte = getRTEForSpecialRelationTypes(pstate, rv);

    /* if not found above, must be a table reference */
    if (!rte)
      rte = transformTableEntry(pstate, rv);

    /* assume new rte is at end */
    rtindex = list_length(pstate->p_rtable);
    Assert(rte == rt_fetch(rtindex, pstate->p_rtable));
    *top_rte = rte;
    *top_rti = rtindex;
    *namespace = list_make1(makeDefaultNSItem(rte));
```

# `order by` を実装する

```
lusiadas=# explain select * from films;
                        QUERY PLAN
----------------------------------------------------------
 Seq Scan on films  (cost=0.00..13.80 rows=380 width=184)
(1 row)

lusiadas=# explain select * from films order by did;
                           QUERY PLAN
----------------------------------------------------------------
 Sort  (cost=30.08..31.03 rows=380 width=184)
   Sort Key: did
   ->  Seq Scan on films  (cost=0.00..13.80 rows=380 width=184)
(3 rows)
```

```
lusiadas=# set session "psql_inspect.planner_script" = 'p PgInspect::PlannedStmt.current_stmt.plan_tree.sort_operators';
SET
lusiadas=# select * from films order by did asc;
# => [97]

lusiadas=# select * from films order by did desc;
# => [521]
```

Oidは"pg_operator.dat"に定義してある数値であり、97は"'less than'"、521は"greater than"となっている。

Plan treeのレベルではSort Nodeがあり、そのlefttreeにSeqScanがある(righttreeはとくにない)。

Execのレベルでは`ExecSort`が処理を行なっており、

> In the sorting operation, if all tuples to be sorted can be stored in work_mem, the quicksort algorithm is used. Otherwise, a temporary file is created and the file merge sort algorithm is used.

`inittapes`

# `group by` を実装する

以下の2つのクエリを考えてみる。

(Query. 1)

```sql
lusiadas=# explain select did from films group by did;
                          QUERY PLAN
--------------------------------------------------------------
 HashAggregate  (cost=14.75..16.75 rows=200 width=4)
   Group Key: did
   ->  Seq Scan on films  (cost=0.00..13.80 rows=380 width=4)
(3 rows)
```

(Query. 2)

```sql
lusiadas=# explain select count(1) from films group by did;
                          QUERY PLAN
--------------------------------------------------------------
 HashAggregate  (cost=15.70..17.70 rows=200 width=12)
   Group Key: did
   ->  Seq Scan on films  (cost=0.00..13.80 rows=380 width=4)
(3 rows)
```

explainの結果は同じだが、plan treeも同じなのだろうか？

```
lusiadas=# set session "psql_inspect.planner_script" = 'p PgInspect::PlannedStmt.current_stmt.plan_tree';

lusiadas=# select did from films group by did;
lusiadas=# select count(1) from films group by did;
```

gram.yではExpr Nodeのlistが作られて、`groupClause`に代入される。

```c
simple_select:
      SELECT opt_all_clause opt_target_list
      into_clause from_clause where_clause
      group_clause having_clause window_clause
        {
          SelectStmt *n = makeNode(SelectStmt);
          n->targetList = $3;
          n->intoClause = $4;
          n->fromClause = $5;
          n->whereClause = $6;
          n->groupClause = $7;
          n->havingClause = $8;
          n->windowClause = $9;
          $$ = (Node *)n;
        }
...

 * Each item in the group_clause list is either an expression tree or a
 * GroupingSet node of some type.
 */
group_clause:
      GROUP_P BY group_by_list        { $$ = $3; }
      | /*EMPTY*/               { $$ = NIL; }
    ;

group_by_list:
      group_by_item             { $$ = list_make1($1); }
      | group_by_list ',' group_by_item   { $$ = lappend($1,$3); }
    ;

group_by_item:
      a_expr                  { $$ = $1; }
      | empty_grouping_set          { $$ = $1; }
      | cube_clause             { $$ = $1; }
      | rollup_clause             { $$ = $1; }
      | grouping_sets_clause          { $$ = $1; }
    ;
```

`transformSelectStmt`では`transformGroupClause`がよばれ、その結果が`groupClause`に代入される。また`groupClause`があるときは、`parseCheckAggregates`がよばれる。

```c
  qry->groupClause = transformGroupClause(pstate,
                      stmt->groupClause,
                      &qry->groupingSets,
                      &qry->targetList,
                      qry->sortClause,
                      EXPR_KIND_GROUP_BY,
                      false /* allow SQL92 rules */ );

...
  if (pstate->p_hasAggs || qry->groupClause || qry->groupingSets || qry->havingQual)
    parseCheckAggregates(pstate, qry);

```

`transformGroupClause`のなかでは`groupClause`をflattenしたのちにイテレートする。今回は`IsA(gexpr, GroupingSet)`ではないケースなので、`transformGroupClauseExpr`がよばれる。

`subquery_planner` -> `grouping_planner` -> `preprocess_groupclause` で `Query->groupClause`に格納。

`UPPERREL_GROUP_AGG` `fetch_upper_rel` `root->upper_rels`

`create_grouping_paths` -> `make_grouping_rel`/`create_ordinary_grouping_paths`
`set_cheapest`

```
typedef struct Path
{
  NodeTag   type;

  NodeTag   pathtype;   /* tag identifying scan/join method */  <- THIS!
```

# 有益コメント集

コメントがかわいい。

```
/*
 * Productions that can be used in both a_expr and b_expr.
 *
 * Note: productions that refer recursively to a_expr or b_expr mostly
 * cannot appear here.  However, it's OK to refer to a_exprs that occur
 * inside parentheses, such as function arguments; that cannot introduce
 * ambiguity to the b_expr syntax.
 */
c_expr:   columnref               { $$ = $1; }
```

うける

```
  /*
   * Special-case "foo = NULL" and "NULL = foo" for compatibility with
   * standards-broken products (like Microsoft's).  Turn these into IS NULL
   * exprs. (If either side is a CaseTestExpr, then the expression was
   * generated internally from a CASE-WHEN expression, and
   * transform_null_equals does not apply.)
   */
```

```
 * When using direct threading, ExecReadyInterpretedExpr will replace
 * each step's opcode field with the address of the relevant code block and
 * ExprState->flags will contain EEO_FLAG_DIRECT_THREADED to remember that
 * that's been done.
```