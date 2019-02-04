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


