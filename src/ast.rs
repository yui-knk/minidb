#[derive(Debug)]
pub enum Stmt {
    // expr, dbname, tablename, where_clause
    SelectStmt(Box<Expr>, String, String, Option<Box<Expr>>),
    // dbname, tablename, keys, values
    InsertStmt(String, String, Vec<String>, Vec<Vec<String>>),
    // dbname, tablename, where_clause
    DeleteStmt(String, String, Option<Box<Expr>>),
}

#[derive(Debug)]
pub enum Expr {
    All,   // "*"
    Count, // "count()"
    Bool(bool),
    Number(i32),
    OpEq(Box<Expr>, Box<Expr>), // "="
    ColumnRef(String), // column name
}
