use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Core types
    Field,
    Bits(Box<Expression>),
    Array {
      element_type: Box<Type>,
      size: usize,
    },
    Nat,
    Bool,
    
    // Type abstractions
    Custom(String),
    GenericType(String),
    
    Unit,
  
    Function {
      params: Vec<Type>,
      return_type: Box<Type>,
    },

    Refined(Box<Type>, Box<Expression>),
    Identifier(String),
    Tuple(Vec<Type>),
}


// #[derive(Debug, Clone, PartialEq)]
// pub enum ConstraintEffect {
//   Unconstrained,
//   Constrained,
// }

impl fmt::Display for Type {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      // Type::Field(ConstraintEffect::Constrained) => write!(f, "field^constrained"),
      // Type::Field => write!(f, "field^unconstrained"),
      Type::Field => write!(f, "field"),
      Type::Bits(size) => write!(f, "Bits<{:?}>", size),
      Type::Array { element_type, size } => write!(f, "Array<{}, {}>", element_type, size),
      Type::Nat => write!(f, "Nat"),
      Type::Bool => write!(f, "Bool"),
      // Type::Bool(ConstraintEffect::Constrained) => write!(f, "Bool^constrained"),
      // Type::Bool(ConstraintEffect::Unconstrained) => write!(f, "Bool^unconstrained"),
      Type::Custom(name) => write!(f, "{}", name),
      Type::GenericType(name) => write!(f, "{}", name),
      Type::Unit => write!(f, "()"),
      Type::Function { params, return_type } => {
        let params_str = params.iter().map(|t| format!("{}", t)).collect::<Vec<_>>().join(", ");
        write!(f, "Function<({}), {}>", params_str, return_type)
      }
      Type::Refined(base, expr) => write!(f, "Refined<{}, {:?}>", base, expr),
      Type::Identifier(name) => write!(f, "{}", name),
      Type::Tuple(types) => {
        let types_str = types.iter().map(|t| format!("{}", t)).collect::<Vec<_>>().join(", ");
        write!(f, "Tuple<{}>", types_str)
      }
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Number(i64),
    Variable(String),
    FunctionCall {
        function: String,
        arguments: Vec<Expression>,
    },
    FunctionDef {
        name: String,
        params: Vec<Parameter>,
        return_type: Type,
        body: Box<Expression>,
    },
    Let {
        pattern: Pattern,
        value: Box<Expression>,
        body: Box<Expression>,
    },
    BinaryOp {
        left: Box<Expression>,
        op: Operator,
        right: Box<Expression>,
    },
    Match {
        value: Box<Expression>,
        patterns: Vec<MatchPattern>,
    },
    Block {
        statements: Vec<Expression>,
        final_expr: Option<Box<Expression>>,
    },
    Component {
        name: String,
        generics: Vec<GenericParam>,
        signals: Vec<Signal>,
        body: Box<Expression>,
    },
    Proof {
        name: String,
        generics: Vec<GenericParam>,
        signals: Vec<Signal>,
        body: Box<Expression>,
    },
    Tuple(Vec<Expression>),
    Assert(Box<Expression>),
    ArrayIndex {
        array: Box<Expression>,
        index: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub name: String,
    pub visibility: Visibility,
    pub typ: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
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
  Variable(String),
  Tuple(Vec<Pattern>),
  Wildcard,
  Constructor(String, Vec<Pattern>),
  Literal(i64)
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
    // Arithmetic
    Add,
    Sub, 
    Mul,
    Div,

    // Comparison
    Equal,    // ==
    NotEqual, // !=
    Gt,       // >
    Lt,       // <
    Ge,       // >=
    Le,       // <=

    // Logical
    And,      // &&
    Or,       // ||
    Not,      // !

    // Constraint
    Assert,   // ===
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
  pub name: String,
  pub bound: Option<Type>
}