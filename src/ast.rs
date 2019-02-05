#[derive(Debug)]
pub enum Stmt {
    // expr, dbname, tablename, where_clause
    SelectStmt(Box<Expr>, String, String, Option<Box<Expr>>),
    // dbname, tablename, keys, values
    InsertStmt(String, String, Vec<String>, Vec<Vec<String>>),
    // dbname, tablename
    DeleteStmt(String, String),
}

#[derive(Debug)]
pub enum Expr {
    All,   // "*"
    Count, // "count()"
    Bool(bool),
    OpEq(Box<Expr>, Box<Expr>), // "="
}
