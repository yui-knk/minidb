use std::rc::Rc;

use config::{Config};
use catalog::catalog::RecordManeger;
use catalog::mini_database::MiniDatabaseRecord;
use catalog::mini_class::MiniClassRecord;
use catalog::mini_attribute::MiniAttributeRecord;

pub struct CatalogManager {
    pub database_rm: RecordManeger<MiniDatabaseRecord>,
    pub class_rm: RecordManeger<MiniClassRecord>,
    pub attribute_rm: RecordManeger<MiniAttributeRecord>,
}

impl CatalogManager {
    pub fn new(config: Rc<Config>) -> CatalogManager {
        CatalogManager {
            database_rm: RecordManeger::mini_database_rm(&config.clone()),
            class_rm: RecordManeger::mini_class_rm(&config.clone()),
            attribute_rm: RecordManeger::mini_attribute_rm(&config.clone()),
        }
    }
}
