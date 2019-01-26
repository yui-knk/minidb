#[derive(Debug)]
pub enum Stmt {
    // expr, dbname, tablename
    SelectStmt(Box<Expr>, String, String),
    // dbname, tablename, keys, values
    InsertStmt(String, String, Vec<String>, Vec<String>),
}

#[derive(Debug)]
pub enum Expr {
    All,   // "*"
    Count,
}
