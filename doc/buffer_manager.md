## buffer page

"src/include/storage/bufpage.h"
`PageHeaderData`がpageのheaderである。これは末尾にflexible arrayをもつ

```c
typedef struct PageHeaderData
{
    ...
    ItemIdData  pd_linp[FLEXIBLE_ARRAY_MEMBER]; /* line pointer array */
} PageHeaderData;
```

```c
typedef Pointer Page;
```

のようにでてくる`Pointer`は"src/include/c.h"に定義されている

```c
typedef char *Pointer;
```

### 初期化

"src/backend/storage/page/bufpage.c"の`PageInit`を見ればわかるように、pageのサイズは常に`BLCKSZ`となっている。`BLCKSZ`は"configure.in"で設定される(コンパイルオプション)であり、デフォルトは8kB(8192B)である。これはpage headerも含めたサイズになる。

## writer process

`BackgroundWriterMain`("src/backend/postmaster/bgwriter.c")により初期化された、writer processは同じ関数のloop内部で、定期的に`BgBufferSync`("src/backend/storage/buffer/bufmgr.c")を実行する。`BgBufferSync`は`SyncOneBuffer`("src/backend/storage/buffer/bufmgr.c")を実行する。

```c
  BufferDesc *bufHdr = GetBufferDescriptor(buf_id);
  /*
   * Pin it, share-lock it, write it.  (FlushBuffer will do nothing if the
   * buffer is clean by the time we've locked it.)
   */
  ...
  PinBuffer_Locked(bufHdr);
  LWLockAcquire(BufferDescriptorGetContentLock(bufHdr), LW_SHARED);

  FlushBuffer(bufHdr, NULL);

  LWLockRelease(BufferDescriptorGetContentLock(bufHdr));
```

### write to file

ここでファイルを実際に書き込むのは`FlushBuffer`("src/backend/storage/buffer/bufmgr.c")である

```c
/*
 * FlushBuffer
 *    Physically write out a shared buffer.
 *
 * NOTE: this actually just passes the buffer contents to the kernel; the
 * real write to disk won't happen until the kernel feels like it.  This
 * is okay from our point of view since we can redo the changes from WAL.
 * However, we will need to force the changes to disk via fsync before
 * we can checkpoint WAL.
 * ...
 */
static void
FlushBuffer(BufferDesc *buf, SMgrRelation reln)
{
...
  /*
   * bufToWrite is either the shared buffer or a copy, as appropriate.
   */
  smgrwrite(reln,
        buf->tag.forkNum,
        buf->tag.blockNum,
        bufToWrite,
        false);
...
}
```

`smgrwrite`("src/backend/storage/smgr/smgr.c")は`smgrsw[reln->smgr_which]`という関数テーブルから`smgr_write`をひいてきてcallする(`smgr_which`は現状`0`で固定されている)。`smgr_write`の実体は`mdwrite`である

```c
void
smgrwrite(SMgrRelation reln, ForkNumber forknum, BlockNumber blocknum,
      char *buffer, bool skipFsync)
{
  smgrsw[reln->smgr_which].smgr_write(reln, forknum, blocknum,
                    buffer, skipFsync);
}
```

`mdwrite`は`FileWrite`を呼び出す、このとき第三引数の`amount`は`BLCKSZ`で固定されている

```c
nbytes = FileWrite(v->mdfd_vfd, buffer, BLCKSZ, WAIT_EVENT_DATA_FILE_WRITE);
```

`FileWrite`("src/backend/storage/file/fd.c")は最終的に`write`(Linuxなら"unistd.h")を呼び出し書き込みを行う

```c
returnCode = write(vfdP->fd, buffer, amount);
```

# Buffer Manager

## Buffer Manager 関連の初期化

`CreateSharedMemoryAndSemaphores`("src/backend/storage/ipc/ipci.c")が呼び出している
`InitBufferPool`("src/backend/storage/buffer/buf_init.c")をみる。

## 

BufferTag により対応するファイルが完全に識別できる。

```c
typedef struct buftag
{
  RelFileNode rnode;      /* physical relation identifier */
  ForkNumber  forkNum;
  BlockNumber blockNum;   /* blknum relative to begin of reln */
} BufferTag;
```

`RelFileNode`は`SMgrRelation`などでも使用されている構造体で、relationの物理層(blockなどは含まない)にアクセスするのに必要な情報を格納している

```c
/*
 * RelFileNode must provide all that we need to know to physically access
 * a relation, with the exception of the backend ID, which can be provided
 * separately. Note, however, that a "physical" relation is comprised of
 * multiple files on the filesystem, as each fork is stored as a separate
 * file, and each fork can be divided into multiple segments. See md.c.
 * ...
 */
typedef struct RelFileNode
{
  Oid     spcNode;    /* tablespace */
  Oid     dbNode;     /* database */
  Oid     relNode;    /* relation */
} RelFileNode;
```

```c
typedef struct BufferDesc
{
  BufferTag tag;      /* ID of page contained in buffer */
  int     buf_id;     /* buffer's index number (from 0) */

  /* state of the tag, containing flags, refcount and usagecount */
  pg_atomic_uint32 state;

  int     wait_backend_pid; /* backend PID of pin-count waiter */
  int     freeNext;   /* link in freelist chain */

  LWLock    content_lock; /* to lock access to buffer contents */
} BufferDesc;
```

実際は`BufferDescPadded`というpaddingつきの構造体を用いる

```c
typedef union BufferDescPadded
{
  BufferDesc  bufferdesc;
  char    pad[BUFFERDESC_PAD_TO_SIZE];
} BufferDescPadded;
```

Buffer Managerそのものを表す構造体はpgには存在しない。
`BufferDescriptors`がbuffer descriptors、`BufferBlocks`がbuffer poolに、`SharedBufHash`がbuffre tableに相当する (http://www.interdb.jp/pg/pgsql08.html)

```c
void
InitBufferPool(void)
{
  bool    foundBufs,
        foundDescs,
        foundIOLocks,
        foundBufCkpt;

  /* Align descriptors to a cacheline boundary. */
  BufferDescriptors = (BufferDescPadded *)
    ShmemInitStruct("Buffer Descriptors",
            NBuffers * sizeof(BufferDescPadded),
            &foundDescs);

  BufferBlocks = (char *)
    ShmemInitStruct("Buffer Blocks",
            NBuffers * (Size) BLCKSZ, &foundBufs);
```

```c
void
InitBufTable(int size)
{
  HASHCTL   info;

  /* assume no locking is needed yet */

  /* BufferTag maps to Buffer */
  info.keysize = sizeof(BufferTag);
  info.entrysize = sizeof(BufferLookupEnt);
  info.num_partitions = NUM_BUFFER_PARTITIONS;

  SharedBufHash = ShmemInitHash("Shared Buffer Lookup Table",
                  size, size,
                  &info,
                  HASH_ELEM | HASH_BLOBS | HASH_PARTITION);
}
```

実際にpageを読み込む箇所は`ReadBuffer_common`を参照。

```c
/*
 * ReadBuffer_common -- common logic for all ReadBuffer variants
 *
 * *hit is set to true if the request was satisfied from shared buffer cache.
 */
static Buffer
ReadBuffer_common(SMgrRelation smgr, char relpersistence, ForkNumber forkNum,
          BlockNumber blockNum, ReadBufferMode mode,
          BufferAccessStrategy strategy, bool *hit)
```

pageにアクセスしたいworkerはBufferTagを渡して、(pageへの参照に該当する)buffer_idを取得する。workerとbuffer managerがやり取りする際に使用するbuffer_idは`Buffer`である。

```c
/*
 * Buffer identifiers.
 *
 * Zero is invalid, positive is the index of a shared buffer (1..NBuffers),
 * negative is the index of a local buffer (-1 .. -NLocBuffer).
 */
typedef int Buffer;
```

`LocalBufferAlloc`

```c
  /*
   * lazy memory allocation: allocate space on first use of a buffer.
   */
  if (LocalBufHdrGetBlock(bufHdr) == NULL)
  {
    /* Set pointer for use by BufferGetBlock() macro */
    LocalBufHdrGetBlock(bufHdr) = GetLocalBufferStorage();
  }
```

# Relation もしくは RelationData

ポスグレにはRelation Cacheという仕組みがある。これはRelationのOidをキーにしてRelationデータをキャッシュするものである。
Lookup用の関数である`RelationIdGetRelation`の定義を参照。

```c
Relation
RelationIdGetRelation(Oid relationId)
```

Relationという型は`RelationData *`のことである。

```c
typedef struct RelationData *Relation;
```

ここで`RelationData`の定義を見てみると、これがとにかくでかい。。。
この構造体が面白いのはStorage Manager用のデータである`SMgrRelationData`がこの構造体のメンバーになっていることである。見落としていなけれ`SMgrRelationData`は唯一この構造体にのみ保持されている。
`RelationData`を作成するための関数は`RelationBuildDesc`である。この時点では`relation->rd_smgr = NULL;`で初期化される。`rd_smgr`は`RelationOpenSmgr`でセットされる。`RelationOpenSmgr`は例えば`ReadBufferExtended`の先頭などで呼ばれるので、`ReadBufferExtended`や`ReadBuffer`を呼び出す側は引数の`Relation`に`rd_smgr`が紐づいているか否かを意識せずに呼び出すことができる。

```c
typedef struct RelationData
{
  RelFileNode rd_node;    /* relation physical identifier */
  /* use "struct" here to avoid needing to include smgr.h: */
  struct SMgrRelationData *rd_smgr; /* cached file handle, or NULL */
  int     rd_refcnt;    /* reference count */
  BackendId rd_backend;   /* owning backend id, if temporary relation */
  bool    rd_islocaltemp; /* rel is a temp rel of this session */
  bool    rd_isnailed;  /* rel is nailed in cache */
  bool    rd_isvalid;   /* relcache entry is valid */
  char    rd_indexvalid;  /* state of rd_indexlist: 0 = not valid, 1 =
                 * valid, 2 = temporarily forced */
  bool    rd_statvalid; /* is rd_statlist valid? */

  /*
   * rd_createSubid is the ID of the highest subtransaction the rel has
   * survived into; or zero if the rel was not created in the current top
   * transaction.  This can be now be relied on, whereas previously it could
   * be "forgotten" in earlier releases. Likewise, rd_newRelfilenodeSubid is
   * the ID of the highest subtransaction the relfilenode change has
   * survived into, or zero if not changed in the current transaction (or we
   * have forgotten changing it). rd_newRelfilenodeSubid can be forgotten
   * when a relation has multiple new relfilenodes within a single
   * transaction, with one of them occurring in a subsequently aborted
   * subtransaction, e.g. BEGIN; TRUNCATE t; SAVEPOINT save; TRUNCATE t;
   * ROLLBACK TO save; -- rd_newRelfilenode is now forgotten
   */
  SubTransactionId rd_createSubid;  /* rel was created in current xact */
  SubTransactionId rd_newRelfilenodeSubid;  /* new relfilenode assigned in
                         * current xact */

  Form_pg_class rd_rel;   /* RELATION tuple */
  TupleDesc rd_att;     /* tuple descriptor */
  Oid     rd_id;      /* relation's object id */
  LockInfoData rd_lockInfo; /* lock mgr's info for locking relation */
  RuleLock   *rd_rules;   /* rewrite rules */
  MemoryContext rd_rulescxt;  /* private memory cxt for rd_rules, if any */
  TriggerDesc *trigdesc;    /* Trigger info, or NULL if rel has none */
  /* use "struct" here to avoid needing to include rowsecurity.h: */
  struct RowSecurityDesc *rd_rsdesc;  /* row security policies, or NULL */

  /* data managed by RelationGetFKeyList: */
  List     *rd_fkeylist;  /* list of ForeignKeyCacheInfo (see below) */
  bool    rd_fkeyvalid; /* true if list has been computed */

  MemoryContext rd_partkeycxt;  /* private memory cxt for the below */
  struct PartitionKeyData *rd_partkey;  /* partition key, or NULL */
  MemoryContext rd_pdcxt;   /* private context for partdesc */
  struct PartitionDescData *rd_partdesc;  /* partitions, or NULL */
  List     *rd_partcheck; /* partition CHECK quals */

  /* data managed by RelationGetIndexList: */
  List     *rd_indexlist; /* list of OIDs of indexes on relation */
  Oid     rd_oidindex;  /* OID of unique index on OID, if any */
  Oid     rd_pkindex;   /* OID of primary key, if any */
  Oid     rd_replidindex; /* OID of replica identity index, if any */

  /* data managed by RelationGetStatExtList: */
  List     *rd_statlist;  /* list of OIDs of extended stats */

  /* data managed by RelationGetIndexAttrBitmap: */
  Bitmapset  *rd_indexattr; /* columns used in non-projection indexes */
  Bitmapset  *rd_projindexattr; /* columns used in projection indexes */
  Bitmapset  *rd_keyattr;   /* cols that can be ref'd by foreign keys */
  Bitmapset  *rd_pkattr;    /* cols included in primary key */
  Bitmapset  *rd_idattr;    /* included in replica identity index */
  Bitmapset  *rd_projidx;   /* Oids of projection indexes */

  PublicationActions *rd_pubactions;  /* publication actions */

  /*
   * rd_options is set whenever rd_rel is loaded into the relcache entry.
   * Note that you can NOT look into rd_rel for this data.  NULL means "use
   * defaults".
   */
  bytea    *rd_options;   /* parsed pg_class.reloptions */

  /* These are non-NULL only for an index relation: */
  Form_pg_index rd_index;   /* pg_index tuple describing this index */
  /* use "struct" here to avoid needing to include htup.h: */
  struct HeapTupleData *rd_indextuple;  /* all of pg_index tuple */

  /*
   * index access support info (used only for an index relation)
   *
   * Note: only default support procs for each opclass are cached, namely
   * those with lefttype and righttype equal to the opclass's opcintype. The
   * arrays are indexed by support function number, which is a sufficient
   * identifier given that restriction.
   *
   * Note: rd_amcache is available for index AMs to cache private data about
   * an index.  This must be just a cache since it may get reset at any time
   * (in particular, it will get reset by a relcache inval message for the
   * index).  If used, it must point to a single memory chunk palloc'd in
   * rd_indexcxt.  A relcache reset will include freeing that chunk and
   * setting rd_amcache = NULL.
   */
  Oid     rd_amhandler; /* OID of index AM's handler function */
  MemoryContext rd_indexcxt;  /* private memory cxt for this stuff */
  /* use "struct" here to avoid needing to include amapi.h: */
  struct IndexAmRoutine *rd_amroutine;  /* index AM's API struct */
  Oid      *rd_opfamily;  /* OIDs of op families for each index col */
  Oid      *rd_opcintype; /* OIDs of opclass declared input data types */
  RegProcedure *rd_support; /* OIDs of support procedures */
  FmgrInfo   *rd_supportinfo; /* lookup info for support procedures */
  int16    *rd_indoption; /* per-column AM-specific flags */
  List     *rd_indexprs;  /* index expression trees, if any */
  List     *rd_indpred;   /* index predicate tree, if any */
  Oid      *rd_exclops;   /* OIDs of exclusion operators, if any */
  Oid      *rd_exclprocs; /* OIDs of exclusion ops' procs, if any */
  uint16     *rd_exclstrats;  /* exclusion ops' strategy numbers, if any */
  void     *rd_amcache;   /* available for use by index AM */
  Oid      *rd_indcollation;  /* OIDs of index collations */

  /*
   * foreign-table support
   *
   * rd_fdwroutine must point to a single memory chunk palloc'd in
   * CacheMemoryContext.  It will be freed and reset to NULL on a relcache
   * reset.
   */

  /* use "struct" here to avoid needing to include fdwapi.h: */
  struct FdwRoutine *rd_fdwroutine; /* cached function pointers, or NULL */

  /*
   * Hack for CLUSTER, rewriting ALTER TABLE, etc: when writing a new
   * version of a table, we need to make any toast pointers inserted into it
   * have the existing toast table's OID, not the OID of the transient toast
   * table.  If rd_toastoid isn't InvalidOid, it is the OID to place in
   * toast pointers inserted into this rel.  (Note it's set on the new
   * version of the main heap, not the toast table itself.)  This also
   * causes toast_save_datum() to try to preserve toast value OIDs.
   */
  Oid     rd_toastoid;  /* Real TOAST table's OID, or InvalidOid */

  /* use "struct" here to avoid needing to include pgstat.h: */
  struct PgStat_TableStatus *pgstat_info; /* statistics collection area */
} RelationData;
```
