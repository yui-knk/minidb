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

## Pageと実ファイルのやりとり

Buffer Manager("bufmgr.c")は`BufferDescriptors`グローバル変数と`BufferBlocks`グローバル変数によってBuffer領域を管理している。一方Storage Manager("smgr.c")は`SMgrRelationHash`ファイルグローバル変数を使って実ファイルのfdを管理している。両者は`RelFileNode`をつかってやりとりをしている。Buffer Managerの場合、`BufferDesc`の`BufferTag`から`RelFileNode`を取得できる。一方Storage Managerの場合、`SMgrRelationHash`のkeyが`RelFileNode`になっている。Buffer領域へのファイルデータの読み込みは`smgrread`、Buffer領域からファイルへのデータの書き込みは`smgrwrite`で行われる。どちらもBuffer側はpageのpointerを渡すようになっている。

## Folk, block, page and segment

```shell
$ cd $PGDATA
$ ls -la base/16384/18751*
-rw------- 1 postgres postgres  8192 Apr 21 10:21 base/16384/18751
-rw------- 1 postgres postgres 24576 Apr 21 10:18 base/16384/18751_fsm
-rw------- 1 postgres postgres  8192 Apr 21 10:18 base/16384/18751_vm

$ ls -la -h base/16384/19427*
-rw------- 1 postgres postgres 1.0G  Apr  21 11:16 base/16384/19427
-rw------- 1 postgres postgres  45M  Apr  21 11:20 base/16384/19427.1
...
```

Folk numberはdata file(0)、free space map file(1)、visibility map file(2)を指し示している。Blockとpageは同じ概念でデフォルト8KB単位のデータのこと。segmentはファイルが大きくなったときに分割したもので、`19427.1`の`1`に相当する。

## BlockNumber

### BlockNumber情報の保存とvalidation

pgではファイルが特定できた状況で当該ファイルのBlock数を調べるのには`_mdnblocks`("md.c")関数を使う。この関数は`smgrnblocks`("smgr.c")、`mdnblocks`("md.c")などを経由して呼ばれる。この関数は単純にファイルを末尾までseekしてファイルサイズを取得し、それを`BLCKSZ`で割ってBlock数を計算する。

```c
static BlockNumber
_mdnblocks(SMgrRelation reln, ForkNumber forknum, MdfdVec *seg)
{
  off_t   len;

  len = FileSeek(seg->mdfd_vfd, 0L, SEEK_END);
  if (len < 0)
    ereport(ERROR,
        (errcode_for_file_access(),
         errmsg("could not seek to end of file \"%s\": %m",
            FilePathName(seg->mdfd_vfd))));
  /* note that this calculation will ignore any partial block at EOF */
  return (BlockNumber) (len / BLCKSZ);
}
```

1ファイルに書き込めるblockの総数はコンパイル時に決定する`RELSEG_SIZE`という値によって制御される。
`_mdnblocks`がsegmentまで決定した状態でBlockNumberを計算するのに対し、`mdnblocks`はForkNumberまでしか決定していない状態でBlockNumberを計算する。そのためsegmentが複数あるときは`_mdnblocks`を複数回呼び出すことになる。

`SMgrRelationData`には`smgr_targblock`というメンバーがあり、この値でForkレベルでみたときの次のinsert先blockを管理している。
`smgropen`で新規に`SMgrRelation`を作成したとき、`smgr_targblock`は`InvalidBlockNumber`で作成される。
`smgr_targblock`へのアクセスは`RelationGetTargetBlock`/`RelationSetTargetBlock`("rel.h")で行う。
`RelationGetBufferForTuple`("hio.c")はtupleを書き込むためのbufferを取得するための関数であるが、ここでは現在の`smgr_targblock`、`FreeSpace`、これから挿入しようとしているtupleの長さなどをもとに`targetBlock`を決定し、`ReadBuffer`によってbufferへの読み込みを行う。そして最後に`smgr_targblock`を更新する。
