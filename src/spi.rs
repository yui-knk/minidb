// `spi.c` in pg.
use std::rc::Rc;

use ast::{Stmt, Expr};
use dml::{InsertIntoCommnad, SelectFromCommnad, CountCommnad};
use tuple::{KeyValueBuilder};
use config::{Config};

lalrpop_mod!(pub parser);

pub struct Executor {
    config: Rc<Config>,
}

impl Executor {
    pub fn new(config: Rc<Config>) -> Executor {
        Executor {
            config: config,
        }
    }

    // See `SPI_execute` in pg.
    pub fn execute_query(&self, query: &str) -> Result<(), String> {
        let parser = parser::StatementParser::new();
        let stmt = parser.parse(query).expect("Invalid syntax");
        
        match stmt {
            Stmt::SelectStmt(expr, dbname, tablename) => {
                match *expr {
                    Expr::All => {
                        let select_from = SelectFromCommnad::new(self.config.clone());
                        select_from.execute(&dbname, &tablename)
                    },
                    Expr::Count => {
                        let count = CountCommnad::new(self.config.clone());
                        count.execute(&dbname, &tablename)
                    },
                }
            },
            Stmt::InsertStmt(dbname, tablename, keys, value_lists) => {
                for values in value_lists.iter() {
                    let mut builder = KeyValueBuilder::new();

                    for (k, v) in keys.iter().zip(values.iter()) {
                        builder.add_pair(k, v)
                    }

                    let insert_into = InsertIntoCommnad::new(self.config.clone());
                    insert_into.execute(&dbname, &tablename, builder.build())?;
                }

                Ok(())
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
    fn insert_stmt() {
        assert!(parser::StatementParser::new().parse("insert into db.tbl (id, age) values (4, 20)").is_ok());
        assert!(parser::StatementParser::new().parse(r#"insert into db.tbl (id, age) values ('a', 'b')"#).is_ok());
        assert!(parser::StatementParser::new().parse(r#"insert into db.tbl (id, age) values ("a", "b")"#).is_ok());
        assert!(parser::StatementParser::new().parse("insert into db.tbl (id, age) values (1, 10), (4, 20)").is_ok());
    }
}
