# Catalog 定義

system catalogの定義は "src/include/catalog/" 以下に置かれている。またbuildをすると "pg_class_d.h" のように "XXX_d.h" というファイルがperlにより生成される。これらのcatalogのうち、一部は "/global" 配下に、また一部は "/base/db_oid/" 以下に配置される。例えば "pg_database" (1262)は "/global" 以下にのみ存在する。一方で "pg_class" (1259)は "/base" 以下のすべてのディレクトリにそれぞれ存在する。

```c
#define DatabaseRelationId 1262
```

```c
#define RelationRelationId 1259
```

