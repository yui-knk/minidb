* Pages read/write database files.
* Tuples read/write a part of page.
* TupleDesc is created on Vec of AttributeRecord and this will have long lifetime because it's valid until the table definition will be changed.

# Ref

* https://www.postgresqlinternals.org
* https://lets.postgresql.jp/node/165
* http://www.interdb.jp/pg/pgsql01.html ("forks")
* http://www.nminoru.jp/~nminoru/postgresql/pg-basic-types-and-tuple.html
* https://pgconf.ru/media/2016/05/13/tuple-internals.pdf
* https://www.sraoss.co.jp/event_seminar/2006/inside.pdf
