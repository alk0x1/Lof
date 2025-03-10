use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Core types
    Field,
    Bits(Box<Expression>),
    Array(Box<Type>, Box<Expression>),
    Nat,
    Bool,
    
    // Type abstractions
    Custom(String),
    GenericType(String),
    
    Unit,
    
    Refined(Box<Type>, Box<Expression>)  // Types with predicates
}

impl fmt::Display for Type {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Type::Field => write!(f, "Field"),
      Type::Bits(size) => write!(f, "Bits<{:?}>", size),
      Type::Array(elem_type, size) => write!(f, "Array<{}, {:?}>", elem_type, size),
      Type::Nat => write!(f, "Nat"),
      Type::Bool => write!(f, "Bool"),
      Type::Custom(name) => write!(f, "{}", name),
      Type::GenericType(name) => write!(f, "{}", name),
      Type::Unit => write!(f, "()"),
      Type::Refined(base, expr) => write!(f, "Refined<{}, {:?}>", base, expr),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
  Number(i64),
  Variable(String),
  BinaryOp {
    left: Box<Expression>,
    op: Operator,
    right: Box<Expression>,
  },
  FunctionCall {
    function: String,
    arguments: Vec<Expression>,
  },
  Block(Vec<Expression>),
  Match {
    value: Box<Expression>,
    patterns: Vec<MatchPattern>,
  },
  Proof {
    name: String,
    generics: Vec<GenericParam>,
    signals: Vec<Signal>,
    constraints: Vec<Constraint>,
  },
  Component {
    name: String,
    generics: Vec<GenericParam>,
    signals: Vec<Signal>,
    constraints: Vec<Constraint>,
  },
  Let {
    name: String,
    value: Box<Expression>,
    body: Box<Expression>,
  },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub name: String,
    pub visibility: Visibility,
    pub typ: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Input,
    Witness,    // implicitly linear
    Output
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
  Constructor(String, Vec<Pattern>),
  Variable(String),
  Number(i64),
  Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchPattern {
  pub pattern: Pattern,
  pub body: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
  Assert(Box<Expression>),
  Verify(Box<Expression>),
  Let(Box<Expression>),
  Match(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Operator {
  Add,
  Sub,
  Mul,
  Div,
  Assert,
  Lt,
  Gt,
  Le,
  Ge,
  Eq,
  Decompose,
  And
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
  pub name: String,
  pub bound: Option<Type>
}