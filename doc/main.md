# Main

`main`("src/backend/main/main.c")がエントリーポイント。

```c
  if (argc > 1 && strcmp(argv[1], "--boot") == 0)
    AuxiliaryProcessMain(argc, argv); /* does not return */
  else if (argc > 1 && strcmp(argv[1], "--describe-config") == 0)
    GucInfoMain();      /* does not return */
  else if (argc > 1 && strcmp(argv[1], "--single") == 0)
    PostgresMain(argc, argv,
           NULL,    /* no dbname */
           strdup(get_user_name_or_exit(progname)));  /* does not return */
  else
    PostmasterMain(argc, argv); /* does not return */
  abort();          /* should not get here */
```

single-user modeとそれ以外で、PostgresMain/PostmasterMainを呼びわけている。

## PostmasterMain

ポスグレはclientからの接続の都度forkをして、子プロセスに処理を委ねる。`PostmasterMain`("src/backend/postmaster/postmaster.c")は`ServerLoop`を呼び出し、その中で`select`を呼んでclientからの接続をまつ。接続があると`BackendStartup`を呼んでbackend processを起動する。子プロセスの場合(`pid == 0`)は`BackendRun`を呼び出すが、この中で`PostgresMain`が呼ばれる。

## PostgresMain

`PostgresMain`("src/backend/tcop/postgres.c")はforによるloopになっている。clientからのリクエストは先頭の文字(`firstchar`)によって処理が振り分けられる。

Ref: https://www.postgresql.org/docs/10/protocol-message-formats.html

クエリ処理('Q')のエントリーポイントは`exec_simple_query`である。ここでは`pg_parse_query`、`pg_analyze_and_rewrite`、`pg_plan_queries`とクエリを処理していく。

```c
  parsetree_list = pg_parse_query(query_string);
  ...
  /*
   * Run through the raw parsetree(s) and process each one.
   */
  foreach(parsetree_item, parsetree_list)
  {
    RawStmt    *parsetree = lfirst_node(RawStmt, parsetree_item);
    ...
    querytree_list = pg_analyze_and_rewrite(parsetree, query_string,
                        NULL, 0, NULL);

    plantree_list = pg_plan_queries(querytree_list,
                    CURSOR_OPT_PARALLEL_OK, NULL);

```

`plantree_list`は`Portal`("portalmem.c")という空間で実行される。`Protal`に関連する処理は以下の順で行う。

* `CreatePortal`: Portalのメモリ確保と初期化
* `PortalDefineQuery`: `portal->stmts`に`plantree_list`をセット
* `PortalStart`: `PORTAL_ONE_SELECT`の場合、ここで`CreateQueryDesc`と`ExecutorStart`を行う
* `PortalSetResultFormat`
* `SetRemoteDestReceiverParams`
* `PortalRun`: `PortalRunSelect`を経て`ExecutorRun`を呼び出したり呼び出さなかったりする。`PORTAL_ONE_RETURNING`/`PORTAL_ONE_MOD_WITH`の場合、`PortalRunSelect`の前の`FillPortalStore`の呼び出しで、`PortalRunMulti` -> `ProcessQuery`で`CreateQueryDesc`と`ExecutorStart`を行う。

ここで`PORTAL_UTIL_SELECT`について少しふれておくと、これはもともと`ChoosePortalStrategy`という関数でNodeの`commandType`をみて判断している(そもそもの`commandType`は"analyze.c"を参照)。`UtilityReturnsTuples`をみると`T_ExplainStmt`とか`T_VariableShowStmt`が該当することがわかる。

* `PortalDrop`: Destroy the portal.

- `pg_analyze_and_rewrite`
-- `parse_analyze`: "src/backend/parser/analyze.c"。`ParseState`の初期化。`RawStmt`から`Query`(Query node/Query tree)を生成する。呼び出す関数は`transformTopLevelStmt`など`transformXXX`。最後に`ParseState`を解放する。
- `pg_plan_queries`
-- `pg_plan_query`: "src/backend/tcop/postgres.c"。`Query`から`PlannedStmt`を生成する。
--- `planner`: "src/backend/optimizer/plan/planner.c"。
---- `standard_planner`: `makeNode(PlannerGlobal)`で`PlannerGlobal`という特殊なNodeを確保。
----- `subquery_planner`: 呼び出しの都度`root = makeNode(PlannerInfo)`で`PlannerInfo` Nodeを作成。この`root`がこの関数の戻り値になる。関数の引数は`PlannerGlobal`や`Query`。

# Query Tree から Plan Treeをつくる

http://www.interdb.jp/pg/pgsql03.html#_3.3.

- `standard_planner`
-- `subquery_planner`
--- `preprocess_expression`
---- `eval_const_expressions`

"3.3.1. Preprocessing"に書かれている定数計算やBooleanの計算は`eval_const_expressions`で行われる。`subquery_planner`の中にある"Now that we are done preprocessing expressions,"のコメントの箇所でpreprocesが終わる。ここからaccess pathの計算に入っていく。

- `subquery_planner`
-- `grouping_planner`: group byの有無に関わらずこのなかでaccess pathを計算している。
--- `preprocess_targetlist`: 単純なケースでは`parse->targetList`(つまり`simple_select:`の`opt_target_list`)を返す。
--- `query_planner`
---- `setup_simple_rel_arrays`
---- `add_base_rels_to_query`
----- `build_simple_rel` (1)
---- `deconstruct_jointree`
----- `deconstruct_recurse`
------ `distribute_qual_to_rels`
------- `distribute_restrictinfo_to_rels` (2)
---- `qp_callback` (== `standard_qp_callback`) (3)
----- `make_pathkeys_for_sortclauses`
------ `get_sortgroupclause_expr`
------- `get_sortgroupclause_tle`
-------- `get_sortgroupref_tle`
------ `make_pathkey_from_sortop`
------- `make_pathkey_from_sortinfo`
-------- `get_eclass_for_sort_expr`: `PlannerInfo->eq_classes`から該当する`EquivalenceClass`をさがして見つかったら返す。見つからないときは作成する。
--------- `add_eq_member`
---- `make_one_rel`
----- `set_base_rel_pathlists`
------ `set_rel_pathlist`: `set_rel_pathlist_hook` is called
------- `set_plain_rel_pathlist`
-------- `create_seqscan_path`
----- `make_rel_from_joinlist`
------ `standard_join_search`
--- `create_pathtarget`
--- `create_grouping_paths`
---- `make_grouping_rel`
---- `create_ordinary_grouping_paths`
----- `add_paths_to_grouping_rel`
------ `add_path` and `create_agg_path`
--- `create_ordered_paths`

FromとWhereは`Query->jointree`に入っていることに注意しながら`query_planner`を読んでいく。まずwhereのないケース(`parse->jointree->fromlist == NIL`)で分岐する。ここではwhereはあるので先にすすむ。
`PlannerInfo *root`のjoinやrelsに関する情報を初期化する。
`setup_simple_rel_arrays`で`Query->rtable`の長さをもとに`PlannerInfo->simple_rel_array`と`PlannerInfo->simple_rte_array`を初期化する。そして`PlannerInfo->simple_rte_array`の中身に`Query->rtable`の要素を代入していく。
`add_base_rels_to_query`ではFromおよびJoinを`RangeTblRef`になるまで再帰的にたどる。`RangeTblRef`はこのクエリで使用されるRelationたちのことであり、`transformSelectStmt`などanalyzeフェーズで作られる。
`build_simple_rel`では`setup_simple_rel_arrays`でつくった`simple_rte_array`から`RangeTblEntry`を取得し、`RelOptInfo`を作成し、`PlannerInfo->simple_rel_array`に登録する。ここではまだwhere(`FromExpr->quals`)を`RelOptInfo->baserestrictinfo`に代入していない。
`deconstruct_jointree`では`Query->jointree`に対して`deconstruct_recurse`が呼び出される。
`deconstruct_recurse`では対象のNodeが`FromExpr`のケースでは`FromExpr->quals`をイテレートして`distribute_qual_to_rels`を呼び出す。`deconstruct_recurse`はNodeがRangeTblRefのときに止まる再帰関数であり、戻り値は`RangeTblRef`のlistとなる(JOIN_FULLのときだけはlistがネストする)。大元の呼び出し元である`query_planner`ではこのlistを`joinlist`に代入し、その後の処理で使う。
`distribute_qual_to_rels`では`make_restrictinfo`をよんで`RestrictInfo`を作成し、
`distribute_restrictinfo_to_rels`を呼び出して適切な`RelOptInfo->baserestrictinfo`に`RestrictInfo`をセットする。
`qp_callback`では必要に応じて`PlannerInfo`の`group_pathkeys`、`window_pathkeys`、`distinct_pathkeys`を計算する。`sort_pathkeys`は必ず計算される。計算には`make_pathkeys_for_sortclauses`を用いる。
`make_one_rel`では全ての`access paths`を計算し、
`make_rel_from_joinlist`の引数はもとをたどれば`deconstruct_recurse`の戻り値であり、それは`Query->jointree`をベースにしている。そのため`joinlist`の要素は`RangeTblRef`もしくは`RangeTblRef`のlistである。まず`joinlist`をもとに`RelOptInfo`のlistをつくる(これは`PlannerInfo->simple_rel_array`を使えば可能)。

`RelOptInfo`: `PlannerInfo->upper_rels`は`RelOptInfo`の配列の配列を所持している。
`PathTarget`: 
`Path`: `create_plan_recurse`では`Path->pathtype`で分岐してPlanを作成する。`get_cheapest_fractional_path`で決定するのはこの構造体。

```c
    /* Preprocess targetlist */
    tlist = preprocess_targetlist(root);

    /*
     * We are now done hacking up the query's targetlist.  Most of the
     * remaining planning work will be done with the PathTarget
     * representation of tlists, but save aside the full representation so
     * that we can transfer its decoration (resnames etc) to the topmost
     * tlist of the finished Plan.
     */
    root->processed_tlist = tlist;

    ...

    /*
     * Convert the query's result tlist into PathTarget format.
     *
     * Note: it's desirable to not do this till after query_planner(),
     * because the target width estimates can use per-Var width numbers
     * that were obtained within query_planner().
     */
    final_target = create_pathtarget(root, tlist);
```

`preprocess_targetlist`では`Query->targetList`をもとにする。遡ると`SelectStmt->targetList`で、これはgram.yの`opt_target_list`だったりする。gram.y上での要素は`target_el:`の`makeNode(ResTarget)`。

