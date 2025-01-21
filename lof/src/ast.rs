#[derive(Debug, Clone, PartialEq)]
pub enum Type {
  Field,
  FieldRange(Box<Expression>, Box<Expression>),
  Bits(Box<Expression>),
  Array(Box<Type>, Box<Expression>),
  Nat,
  Bool,
  Custom(String),
  GenericType(String),
  Unit,
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
  Witness,
  Output,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchPattern {
  pub pattern: Pattern,
  pub body: Box<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
  Constructor(String, Vec<Pattern>),
  Variable(String),
  Number(i64),
  Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
  Assert(Box<Expression>),
  Verify(Box<Expression>),
  Let(Box<Expression>),
  Match(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
  pub name: String,
  pub bound: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
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
}