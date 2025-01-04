use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
  Nat,
  Bool,
  Private(Box<Type>),
  Public(Box<Type>),
}

#[derive(Debug, Clone)]
pub enum Expression {
  Number(i64),
  Variable(String),
  BinaryOp {
    left: Box<Expression>,
    operator: Operator,
    right: Box<Expression>,
  },
  Theorem {
    name: String,
    inputs: Vec<Parameter>,
    output: Type,
    body: Box<Expression>,
  },
}

#[derive(Debug, Clone)]
pub struct Parameter {
  pub name: String,
  pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum Operator {
  Add,
  Multiply,
  LessThan,
  Equals,
}

impl fmt::Display for Type {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Type::Nat => write!(f, "Nat"),
      Type::Bool => write!(f, "Bool"),
      Type::Private(t) => write!(f, "Private {}", t),
      Type::Public(t) => write!(f, "Public {}", t),
    }
  }
}

impl fmt::Display for Expression {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Expression::Number(n) => write!(f, "{}", n),
      Expression::Variable(name) => write!(f, "{}", name),
      Expression::BinaryOp { left, operator, right } => {
        write!(f, "({} {} {})", left, operator, right)
      },
      Expression::Theorem { name, inputs, output, body } => {
        writeln!(f, "theorem {}", name)?;
        for param in inputs {
          writeln!(f, "  {}", param)?;
        }
        writeln!(f, "  : {}", output)?;
        write!(f, "  {}", body)
      }
    }
  }
}

impl fmt::Display for Parameter {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "({}: {})", self.name, self.typ)
  }
}

impl fmt::Display for Operator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Operator::Add => write!(f, "+"),
      Operator::Multiply => write!(f, "*"),
      Operator::LessThan => write!(f, "<"),
      Operator::Equals => write!(f, "="),
    }
  }
}

impl Expression {
  pub fn print_tree(&self) -> String {
    self.print_tree_helper("", "")
  }

  fn print_tree_helper(&self, prefix: &str, _: &str) -> String {
    let mut result = String::new();
    match self {
      Expression::Number(n) => {
        result.push_str(&format!("{}└── Number({})\n", prefix, n));
      },
      Expression::Variable(name) => {
        result.push_str(&format!("{}└── Variable({})\n", prefix, name));
      },
      Expression::BinaryOp { left, operator, right } => {
        result.push_str(&format!("{}└── BinaryOp({})\n", prefix, operator));
        result.push_str(&left.print_tree_helper(&format!("{}  ", prefix), ""));
        result.push_str(&right.print_tree_helper(&format!("{}  ", prefix), ""));
      },
      Expression::Theorem { name, inputs, output, body } => {
        result.push_str(&format!("{}└── Theorem({})\n", prefix, name));
        result.push_str(&format!("{}  ├── Inputs:\n", prefix));
        for param in inputs {
          result.push_str(&format!("{}  │   └── Parameter({}: {})\n", prefix, param.name, param.typ));
        }
        result.push_str(&format!("{}  ├── Output: {}\n", prefix, output));
        result.push_str(&format!("{}  └── Body:\n", prefix));
        result.push_str(&body.print_tree_helper(&format!("{}    ", prefix), ""));
      }
    }
    result
  }
}
