// `spi.c` in pg.
use std::rc::Rc;

use ast::{Stmt, Expr};
use dml::{InsertIntoCommnad, SelectFromCommnad, CountCommnad, DeleteCommnad};
use tuple::{KeyValueBuilder};
use config::{Config};
use catalog::catalog_manager::CatalogManager;

lalrpop_mod!(pub parser);

pub struct Executor<'a> {
    config: Rc<Config>,
    cmgr: &'a CatalogManager
}

impl<'a> Executor<'a> {
    pub fn new(config: Rc<Config>, cmgr: &'a CatalogManager) -> Executor<'a> {
        Executor {
            config: config,
            cmgr: cmgr
        }
    }

    // See `SPI_execute` in pg.
    pub fn execute_query(&self, query: &str) -> Result<(), String> {
        let parser = parser::StatementParser::new();
        let stmt = parser.parse(query).expect("Invalid syntax");
        
        match stmt {
            Stmt::SelectStmt(expr, dbname, tablename, where_clause) => {
                match *expr {
                    Expr::All => {
                        let select_from = SelectFromCommnad::new(self.config.clone());
                        select_from.execute(&dbname, &tablename, self.cmgr, &where_clause)
                    },
                    Expr::Count => {
                        let count = CountCommnad::new(self.config.clone());
                        count.execute(&dbname, &tablename, self.cmgr, &where_clause)
                    },
                    Expr::Bool(_) => {
                        panic!("Unknown expr ({:?})", expr);
                    },
                    Expr::Number(_) => {
                        panic!("Unknown expr ({:?})", expr);
                    },
                    Expr::OpEq(..) => {
                        panic!("Unknown expr ({:?})", expr);
                    },
                    Expr::ColumnRef(..) => {
                        panic!("Unknown expr ({:?})", expr);
                    },
                }
            },
            Stmt::InsertStmt(dbname, tablename, keys, value_lists) => {
                // TODO: Implement nodeValuesscan and change InsertIntoCommnad
                //       to fetch all records.
                for values in value_lists.iter() {
                    let mut builder = KeyValueBuilder::new();

                    for (k, v) in keys.iter().zip(values.iter()) {
                        builder.add_pair(k, v)
                    }

                    let insert_into = InsertIntoCommnad::new(self.config.clone());
                    insert_into.execute(&dbname, &tablename, builder.build(), self.cmgr)?;
                }

                Ok(())
            },
            Stmt::DeleteStmt(dbname, tablename) => {
                let delete = DeleteCommnad::new(self.config.clone());
                delete.execute(&dbname, &tablename, self.cmgr)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    lalrpop_mod!(pub parser);

    #[test]
    fn select_stmt() {
        assert!(parser::StatementParser::new().parse("select * from db.tbl").is_ok());
        assert!(parser::StatementParser::new().parse("select count() from db.tbl").is_ok());
    }

    #[test]
    fn select_stmt_with_where() {
        assert!(parser::StatementParser::new().parse("select * from db.tbl where true").is_ok());
        assert!(parser::StatementParser::new().parse("select * from db.tbl where false").is_ok());
        assert!(parser::StatementParser::new().parse("select * from db.tbl where false = true").is_ok());
    }

    #[test]
    fn insert_stmt() {
        assert!(parser::StatementParser::new().parse("insert into db.tbl (id, age) values (4, 20)").is_ok());
        assert!(parser::StatementParser::new().parse(r#"insert into db.tbl (id, age) values ('a', 'b')"#).is_ok());
        assert!(parser::StatementParser::new().parse(r#"insert into db.tbl (id, age) values ("a", "b")"#).is_ok());
        assert!(parser::StatementParser::new().parse("insert into db.tbl (id, age) values (1, 10), (4, 20)").is_ok());
    }

    #[test]
    fn delete_stmt() {
        assert!(parser::StatementParser::new().parse("delete from db.tbl").is_ok());
    }
}
