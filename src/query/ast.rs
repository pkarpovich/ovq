#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Compare {
        field: String,
        op: CompareOp,
        value: Value,
    },
    Contains {
        field: String,
        value: Value,
    },
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Date(Date),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }
}
