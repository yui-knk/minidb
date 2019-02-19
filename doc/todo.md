# TODO

* [x] Use Oid (unsigned int) in RelFileNode instead of String.
* [x] Implement Storage Manager
* [x] Multi pages support
* [x] Count function support
* [x] Implement parser
* [x] Support multi rows insert syntax
* [x] Delete operation support
* [x] Support where condition clause for select
* [x] Support where condition clause for delete
* [x] Trait for node executions
* [ ] Update operation support
* [ ] Index support
* [ ] Support where condition clause for update
* [ ] Implement nice internal value representation
* [ ] Add dirty flag to pages to avoid needless writing when drop the page
* [ ] Implement ItemPointerData (tuple id) in HeapTupleHeaderData to support delete operation
* [ ] Manual vacuum support
* [ ] Query (Query node/Query tree) and "parse_analyze"
* [ ] Implement plan tree builder
* [ ] Null value support
* [ ] WAL support
* [ ] Multi segment support
* [ ] Join support
* [ ] Group by support
