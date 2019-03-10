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

## 初期化

`CreateSharedMemoryAndSemaphores`("src/backend/storage/ipc/ipci.c")
`InitBufferPool`("src/backend/storage/buffer/buf_init.c")

# Buffer Manager

buffer_tagにより対応するファイルが完全に識別できる。

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
