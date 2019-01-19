# Storage Manager

* "src/backend/storage/smgr/smgr.c"
* "src/include/storage/smgr.h"

## SMgrRelation

"rel.h"をみると`RelationData`というRelationを管理するための構造体がみつかる。この構造体のメンバーに`rd_smgr`というのがある。これはRelationに対応するfileを管理するための機構であり、Storage Manager (smgr)と呼ばれる。

```c
/*
 * Here are the contents of a relation cache entry.
 */

typedef struct RelationData
{
  RelFileNode rd_node;    /* relation physical identifier */
  /* use "struct" here to avoid needing to include smgr.h: */
  struct SMgrRelationData *rd_smgr; /* cached file handle, or NULL */
  int     rd_refcnt;    /* reference count */
```

`SMgrRelationData`は以下のような構造体になっている。

```c
typedef struct SMgrRelationData
{
  /* rnode is the hashtable lookup key, so it must be first! */
  RelFileNodeBackend smgr_rnode;  /* relation physical identifier */

  /* pointer to owning pointer, or NULL if none */
  struct SMgrRelationData **smgr_owner;

  /*
   * These next three fields are not actually used or manipulated by smgr,
   * except that they are reset to InvalidBlockNumber upon a cache flush
   * event (in particular, upon truncation of the relation).  Higher levels
   * store cached state here so that it will be reset when truncation
   * happens.  In all three cases, InvalidBlockNumber means "unknown".
   */
  BlockNumber smgr_targblock; /* current insertion target block */
  BlockNumber smgr_fsm_nblocks; /* last known size of fsm fork */
  BlockNumber smgr_vm_nblocks;  /* last known size of vm fork */

  /* additional public fields may someday exist here */

  /*
   * Fields below here are intended to be private to smgr.c and its
   * submodules.  Do not touch them from elsewhere.
   */
  int     smgr_which;   /* storage manager selector */

  /*
   * for md.c; per-fork arrays of the number of open segments
   * (md_num_open_segs) and the segments themselves (md_seg_fds).
   */
  int     md_num_open_segs[MAX_FORKNUM + 1];
  struct _MdfdVec *md_seg_fds[MAX_FORKNUM + 1];

  /* if unowned, list link in list of all unowned SMgrRelations */
  struct SMgrRelationData *next_unowned_reln;
} SMgrRelationData;
```

"src/backend/storage/smgr/md.c"のコメントにあるように File descriptor (fd) は`md_seg_fds`にforkごとに格納されている。forkについては http://www.interdb.jp/pg/pgsql01.html を参照。

```c
 *  File descriptors are stored in the per-fork md_seg_fds arrays inside
 *  SMgrRelation. The length of these arrays is stored in md_num_open_segs.
```

`RelFileNodeBackend`は`RelFileNode`にBackendの識別子が付与されたものである。

```c
typedef struct RelFileNodeBackend
{
  RelFileNode node;
  BackendId backend;
} RelFileNodeBackend;
```

`SMgrRelation`は`RelFileNode`をkey、`SMgrRelation`をvalueにした`SMgrRelationHash`というhashによって管理される。`SMgrRelationHash`の初期化は`smgropen`で行われる。

```c
/*
 * Each backend has a hashtable that stores all extant SMgrRelation objects.
 * In addition, "unowned" SMgrRelation objects are chained together in a list.
 */
static HTAB *SMgrRelationHash = NULL;
```

`hash_search`は第三引数の`HASHACTION action`によって挙動が変わるが、`HASH_ENTER`のときは必要におうじて新しくhashのエントリ用の領域を確保して、そのポインタを返す。そのため呼び出し側はそのポインタの指す領域にvalueに相当するデータを書いていけばよい。

smgrは抽象化レイヤーになっており、その操作は`typedef struct f_smgr`および`static const f_smgr smgrsw[]`によって行う。`f_smgr`は関数の集合であり、`SMgrRelation`を受け取ってそれぞれの操作を行う。例えばファイルのopenであれば、`smgr_create`メンバーがその処理を行うので、`mdcreate`を読めばよい。

