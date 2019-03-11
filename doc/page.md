## TupleTableSlot

Postgresではtableの行のことを(internalに)Tupleと呼んでいる。中心的なデータ構造は`HeapTupleData`("src/include/access/htup.h")と`HeapTupleHeaderData `("src/include/access/htup_details.h")である。`HeapTupleHeaderData`は`t_bits`のあとに、さらにタプルの実データ用の領域をもつようにメモリが確保される(`heap_form_tuple`("src/backend/access/common/heaptuple.c")参照)。
`HeapTupleFields`にはMVCC用のデータである`t_xmin`/`t_xmax`などがある。

```c
typedef struct HeapTupleData
{
  uint32    t_len;      /* length of *t_data */
  ItemPointerData t_self;   /* SelfItemPointer */
  Oid     t_tableOid;   /* table the tuple came from */
#define FIELDNO_HEAPTUPLEDATA_DATA 3
  HeapTupleHeader t_data;   /* -> tuple header and data */
} HeapTupleData;

typedef HeapTupleData *HeapTuple;
typedef HeapTupleHeaderData *HeapTupleHeader;

struct HeapTupleHeaderData
{
  union
  {
    HeapTupleFields t_heap;
    DatumTupleFields t_datum;
  }     t_choice;

  ItemPointerData t_ctid;   /* current TID of this or newer tuple (or a
                 * speculative insertion token) */

  /* Fields below here must match MinimalTupleData! */

#define FIELDNO_HEAPTUPLEHEADERDATA_INFOMASK2 2
  uint16    t_infomask2;  /* number of attributes + various flags */

#define FIELDNO_HEAPTUPLEHEADERDATA_INFOMASK 3
  uint16    t_infomask;   /* various flag bits, see below */

#define FIELDNO_HEAPTUPLEHEADERDATA_HOFF 4
  uint8   t_hoff;     /* sizeof header incl. bitmap, padding */

  /* ^ - 23 bytes - ^ */

#define FIELDNO_HEAPTUPLEHEADERDATA_BITS 5
  bits8   t_bits[FLEXIBLE_ARRAY_MEMBER];  /* bitmap of NULLs */

  /* MORE DATA FOLLOWS AT END OF STRUCT */
};

typedef struct HeapTupleFields
{
  TransactionId t_xmin;   /* inserting xact ID */
  TransactionId t_xmax;   /* deleting or locking xact ID */

  union
  {
    CommandId t_cid;    /* inserting or deleting command ID, or both */
    TransactionId t_xvac; /* old-style VACUUM FULL xact ID */
  }     t_field3;
} HeapTupleFields;
```

```c
HeapTuple
heap_form_tuple(TupleDesc tupleDescriptor,
        Datum *values,
        bool *isnull)
{
...
  /*
   * Determine total space needed
   */
  len = offsetof(HeapTupleHeaderData, t_bits);

  if (hasnull)
    len += BITMAPLEN(numberOfAttributes);

  if (tupleDescriptor->tdhasoid)
    len += sizeof(Oid);

  hoff = len = MAXALIGN(len); /* align user data safely */

  data_len = heap_compute_data_size(tupleDescriptor, values, isnull);

  len += data_len;

  /*
   * Allocate and zero the space needed.  Note that the tuple body and
   * HeapTupleData management structure are allocated in one chunk.
   */
  tuple = (HeapTuple) palloc0(HEAPTUPLESIZE + len);
...
```

* `heap_compute_data_size`のなかで、`pg_attribute.attlen`などをもとにカラムの実データの長さを計算している。

`HeapTuple`ではカラムのメタ情報はもっていない。カラムのメタ情報は別途`TupleDesc`("src/include/access/tupdesc.h")で持っている。

```c
typedef struct tupleDesc
{
  int     natts;      /* number of attributes in the tuple */
  Oid     tdtypeid;   /* composite type ID for tuple type */
  int32   tdtypmod;   /* typmod for tuple type */
  bool    tdhasoid;   /* tuple has oid attribute in its header */
  int     tdrefcount;   /* reference count, or -1 if not counting */
  TupleConstr *constr;    /* constraints, or NULL if none */
  /* attrs[N] is the description of Attribute Number N+1 */
  FormData_pg_attribute attrs[FLEXIBLE_ARRAY_MEMBER];
}      *TupleDesc;
```

そして`HeapTuple`と`TupleDesc`の両方を結びつけるための構造が、`TupleTableSlot`("src/include/executor/tuptable.h")である。

```c
typedef struct TupleTableSlot
{
  NodeTag   type;
  bool    tts_isempty;  /* true = slot is empty */
  bool    tts_shouldFree; /* should pfree tts_tuple? */
  bool    tts_shouldFreeMin;  /* should pfree tts_mintuple? */
#define FIELDNO_TUPLETABLESLOT_SLOW 4
  bool    tts_slow;   /* saved state for slot_deform_tuple */
#define FIELDNO_TUPLETABLESLOT_TUPLE 5
  HeapTuple tts_tuple;    /* physical tuple, or NULL if virtual */
#define FIELDNO_TUPLETABLESLOT_TUPLEDESCRIPTOR 6
  TupleDesc tts_tupleDescriptor;  /* slot's tuple descriptor */
  MemoryContext tts_mcxt;   /* slot itself is in this context */
  Buffer    tts_buffer;   /* tuple's buffer, or InvalidBuffer */
#define FIELDNO_TUPLETABLESLOT_NVALID 9
  int     tts_nvalid;   /* # of valid values in tts_values */
#define FIELDNO_TUPLETABLESLOT_VALUES 10
  Datum    *tts_values;   /* current per-attribute values */
#define FIELDNO_TUPLETABLESLOT_ISNULL 11
  bool     *tts_isnull;   /* current per-attribute isnull flags */
  MinimalTuple tts_mintuple;  /* minimal tuple, or NULL if none */
  HeapTupleData tts_minhdr; /* workspace for minimal-tuple-only case */
#define FIELDNO_TUPLETABLESLOT_OFF 14
  uint32    tts_off;    /* saved state for slot_deform_tuple */
  bool    tts_fixedTupleDescriptor; /* descriptor can't be changed */
} TupleTableSlot;
```

https://pgconf.ru/media/2016/05/13/tuple-internals.pdf の"Tuple header"や、`RelationPutHeapTuple`や`heap_fetch`からわかるように、page(disk)に格納されているtupleは`HeapTupleData.t_data`、つまり`HeapTupleHeaderData`構造体の形をしており、実データ以外にinfomaskやtid、null bitmapを先頭に持っているのである。

```c
/*
 * RelationPutHeapTuple - place tuple at specified page
 *
 * !!! EREPORT(ERROR) IS DISALLOWED HERE !!!  Must PANIC on failure!!!
 *
 * Note - caller must hold BUFFER_LOCK_EXCLUSIVE on the buffer.
 */
void
RelationPutHeapTuple(Relation relation,
           Buffer buffer,
           HeapTuple tuple,
           bool token)
{
  Page    pageHeader;
  OffsetNumber offnum;

  /*
   * A tuple that's being inserted speculatively should already have its
   * token set.
   */
  Assert(!token || HeapTupleHeaderIsSpeculative(tuple->t_data));

  /* Add the tuple to the page */
  pageHeader = BufferGetPage(buffer);

  offnum = PageAddItem(pageHeader, (Item) tuple->t_data,
             tuple->t_len, InvalidOffsetNumber, false, true);
```

```c
bool
heap_fetch(Relation relation,
       Snapshot snapshot,
       HeapTuple tuple,
       Buffer *userbuf,
       bool keep_buf,
       Relation stats_relation)
{
...
  /*
   * fill in *tuple fields
   */
  tuple->t_data = (HeapTupleHeader) PageGetItem(page, lp);
  tuple->t_len = ItemIdGetLength(lp);
```

tupleからユーザーデータを取得するには`GETSTRUCT`マクロを使う

```
/*
 * GETSTRUCT - given a HeapTuple pointer, return address of the user data
 */
#define GETSTRUCT(TUP) ((char *) ((TUP)->t_data) + (TUP)->t_data->t_hoff)
```

## tuple削除時の挙動

Ref: http://www.interdb.jp/pg/pgsql05.html#_5.3.

実際のPostgresではMVCCを採用しており、`t_xmax`や現在のtransaction idを使用してtupleの可視性を判断している。詳しい実装は`heap_fetch`、`HeapTupleSatisfiesVisibility`、`HeapTupleSatisfiesMVCC`を参照。
今回は`t_infomask2`の`HEAP_KEYS_UPDATED`のみをつかって削除を実装する。なお更新処理時も追記をする形で更新を実装するので`HEAP_KEYS_UPDATED`をチェックして、当該フラグが立っているtupleを無視するようにすればよい。

## nodeSeqScanにおけるTupleTableSlotの生成と初期化

例としてnodeSeqScanにおけるTupleTableSlotの生成と初期化をみておく。
SeqScanState構造体は`state->ss->ss_ScanTupleSlot`という形で`TupleTableSlot`をもっている。このメンバーの初期化は `ExecInitSeqScan` -> `ExecInitScanTupleSlot` -> `ExecAllocTableSlot` -> `MakeTupleTableSlot` で行われる。
`MakeTupleTableSlot`は引数の`TupleDesc tupleDesc`がNULLか否かで確保するメモリーの大きさが変化する。NULLでないときは`tts_fixedTupleDescriptor`という扱いになり、この関数の内部で`tts_values`および`tts_isnull`が使用するメモリも含めて確保する。
nodeSeqScanのケースでは`ss_currentRelation`から`TupleDesc`が取得できるため、fixedTupleDescriptorとなる。

```c
typedef struct ScanState
{
  PlanState ps;       /* its first field is NodeTag */
  Relation  ss_currentRelation;
  HeapScanDesc ss_currentScanDesc;
  TupleTableSlot *ss_ScanTupleSlot;
} ScanState;

typedef struct SeqScanState
{
  ScanState ss;       /* its first field is NodeTag */
  Size    pscan_len;    /* size of parallel heap scan descriptor */
} SeqScanState;
```

```
  /* and create slot with the appropriate rowtype */
  ExecInitScanTupleSlot(estate, &scanstate->ss,
              RelationGetDescr(scanstate->ss.ss_currentRelation));

void
ExecInitScanTupleSlot(EState *estate, ScanState *scanstate, TupleDesc tupledesc)
{
  scanstate->ss_ScanTupleSlot = ExecAllocTableSlot(&estate->es_tupleTable,
                           tupledesc);
  scanstate->ps.scandesc = tupledesc;
}

TupleTableSlot *
ExecAllocTableSlot(List **tupleTable, TupleDesc desc)
{
  TupleTableSlot *slot = MakeTupleTableSlot(desc);

  *tupleTable = lappend(*tupleTable, slot);

  return slot;
}
```


