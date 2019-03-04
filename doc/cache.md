# System Cache

`static const struct cachedesc cacheinfo[]` というstaticな変数があり、ここにcalalog関係のキャッシュに"必要な"情報が入っている。`InitCatalogCache`では`cacheinfo`をイテレートしながら`static CatCache *SysCache[SysCacheSize];`に情報を詰めていく。このキャッシュとやりとりするときのkeyは`enum SysCacheIdentifier`である。