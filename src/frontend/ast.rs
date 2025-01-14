use std::fmt;

// Types in our language
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
  Field,                          // Basic field element
  FieldRange(Box<Expression>, Box<Expression>),          // Field with range constraints
  Bits(Box<Expression>),         // Bit array with size
  Array(Box<Type>, Box<Expression>), // Array type with size
  Nat,                           // For type-level numbers
  Bool,                          // Boolean type
  Custom(String),                // Custom type (like Tree)
  GenericType(String),           // Generic type parameter
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
  Constructor(String, Vec<Pattern>),  // For enum variants
  Variable(String),                   // Binding pattern
  Number(i64),                        // Literal pattern
  Wildcard,                          // _ pattern
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
  Assert(Box<Expression>),
  Verify(Box<Expression>),
  Let(Box<Expression>),        // Let bindings
  Match(Box<Expression>),      // Match expressions
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
  pub name: String,
  pub bound: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
  // Arithmetic
  Add,
  Sub,
  Mul,
  Div,
  
  // Constraint
  Assert,     // ===
  
  // Comparison
  Lt,
  Gt,
  Le,
  Ge,
  Eq,
}

impl fmt::Display for Type {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Type::Custom(name) => write!(f, "{}", name),
      Type::GenericType(name) => write!(f, "{}", name),
      Type::Field => write!(f, "Field"),
      Type::FieldRange(min, max) => write!(f, "Field<{}..{}>", min, max),
      Type::Bits(size) => write!(f, "Bits<{}>", size),
      Type::Array(t, size) => write!(f, "Array<{}, {}>", t, size),
      Type::Nat => write!(f, "Nat"),
      Type::Bool => write!(f, "Bool"),
    }
  }
}

impl fmt::Display for Expression {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Expression::Block(expressions) => {
        for expr in expressions {
          writeln!(f, "{}", expr)?;
        }
        Ok(())
      },
      Expression::FunctionCall { function, arguments } => {
        write!(f, "{}", function)?;
        for arg in arguments {
          write!(f, " {}", arg)?;
        }
        Ok(())
      },
      Expression::Number(n) => write!(f, "{}", n),
      Expression::Variable(name) => write!(f, "{}", name),
      Expression::BinaryOp { left, op, right } => {
        write!(f, "({} {} {})", left, op, right)
      },
      Expression::Match { value, patterns } => {
        writeln!(f, "match {} {{", value)?;
        for p in patterns {
          writeln!(f, "  {} => {},", p.pattern, p.body)?;
        }
        write!(f, "}}")
      },
      Expression::Proof { name, generics, signals, constraints } => {
        write!(f, "proof {}", name)?;
        if !generics.is_empty() {
          write!(f, "<")?;
          for (i, g) in generics.iter().enumerate() {
            if i > 0 { write!(f, ", ")? }
            write!(f, "{}", g.name)?;
            if let Some(bound) = &g.bound {
              write!(f, ": {}", bound)?;
            }
          }
          write!(f, ">")?;
        }
        writeln!(f, " {{")?;
        for signal in signals {
          writeln!(f, "    {}", signal)?;
        }
        for constraint in constraints {
          writeln!(f, "  {}", constraint)?;
        }
        write!(f, "}}")
      },
      Expression::Component { name, generics: _, signals: _, constraints: _ } => {
        write!(f, "component {}", name)?;
        // Similar to Proof display...
        Ok(())
      },
      Expression::Let { name, value, body } => {
        write!(f, "let {} = {};", name, value)?;
        writeln!(f, " {}", body)?;
        Ok(())
      },
    }
  }
}

impl fmt::Display for Signal {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} {}: {}", self.visibility, self.name, self.typ)
  }
}

impl fmt::Display for Visibility {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Visibility::Input => write!(f, "input"),
      Visibility::Witness => write!(f, "witness"),
      Visibility::Output => write!(f, "output"),
    }
  }
}

impl fmt::Display for Pattern {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Pattern::Constructor(name, patterns) => {
        write!(f, "{}(", name)?;
        for (i, p) in patterns.iter().enumerate() {
          if i > 0 { write!(f, ", ")? }
          write!(f, "{}", p)?;
        }
        write!(f, ")")
      },
      Pattern::Variable(name) => write!(f, "{}", name),
      Pattern::Number(n) => write!(f, "{}", n),
      Pattern::Wildcard => write!(f, "_"),
    }
  }
}

impl fmt::Display for Constraint {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Constraint::Assert(expr) => write!(f, "assert {};", expr),
      Constraint::Verify(expr) => write!(f, "verify {};", expr),
      Constraint::Let(expr) => write!(f, "let {};", expr),
      Constraint::Match(expr) => write!(f, "match {};", expr),
    }
  }
}

impl fmt::Display for Operator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Operator::Add => write!(f, "+"),
      Operator::Sub => write!(f, "-"),
      Operator::Mul => write!(f, "*"),
      Operator::Div => write!(f, "/"),
      Operator::Assert => write!(f, "==="),
      Operator::Lt => write!(f, "<"),
      Operator::Gt => write!(f, ">"),
      Operator::Le => write!(f, "<="),
      Operator::Ge => write!(f, ">="),
      Operator::Eq => write!(f, "="),
    }
  }
}

impl Expression {
  pub fn print_tree_helper(&self, prefix: &str, _: &str) -> String {
    let mut result = String::new();
    match self {
      Expression::Block(expressions) => {
        result.push_str(&format!("{}└── Block\n", prefix));
        for (i, expr) in expressions.iter().enumerate() {
          let is_last = i == expressions.len() - 1;
          let child_prefix = if is_last { "    " } else { "│   " };
          result.push_str(&expr.print_tree_helper(&format!("{}{}", prefix, child_prefix), ""));
        }
      },
      Expression::Number(n) => {
        result.push_str(&format!("{}└── Number({})\n", prefix, n));
      },
      Expression::Variable(name) => {
        result.push_str(&format!("{}└── Variable({})\n", prefix, name));
      },
      Expression::BinaryOp { left, op, right } => {
        result.push_str(&format!("{}└── {}\n", prefix, op));
        result.push_str(&format!("{}    ├── Left:\n", prefix));
        result.push_str(&left.print_tree_helper(&format!("{}    │   ", prefix), ""));
        result.push_str(&format!("{}    └── Right:\n", prefix));
        result.push_str(&right.print_tree_helper(&format!("{}        ", prefix), ""));
      },
      Expression::FunctionCall { function, arguments } => {
        result.push_str(&format!("{}└── FunctionCall({})\n", prefix, function));
        for (i, arg) in arguments.iter().enumerate() {
          let is_last = i == arguments.len() - 1;
          let arg_prefix = if is_last { "    " } else { "│   " };
          result.push_str(&format!("{}    {}── Argument {}:\n", prefix, if is_last { "└" } else { "├" }, i + 1));
          result.push_str(&arg.print_tree_helper(&format!("{}    {}", prefix, arg_prefix), ""));
        }
      },
      Expression::Match { value, patterns } => {
        result.push_str(&format!("{}└── Match Expression\n", prefix));
        result.push_str(&format!("{}    ├── Value:\n", prefix));
        result.push_str(&value.print_tree_helper(&format!("{}    │   ", prefix), ""));
        result.push_str(&format!("{}    └── Patterns:\n", prefix));
        for (i, pattern) in patterns.iter().enumerate() {
          let is_last = i == patterns.len() - 1;
          result.push_str(&format!("{}        {}── Pattern: {}\n", 
            prefix,
            if is_last { "└" } else { "├" },
            pattern.pattern));
          let body_prefix = if is_last {
            format!("{}            ", prefix)
          } else {
            format!("{}        │   ", prefix)
          };
          result.push_str(&pattern.body.print_tree_helper(&body_prefix, ""));
        }
      },
      Expression::Let { name, value, body } => {
        result.push_str(&format!("{}└── Let {}\n", prefix, name));
        result.push_str(&format!("{}    ├── Value:\n", prefix));
        result.push_str(&value.print_tree_helper(&format!("{}    │   ", prefix), ""));
        result.push_str(&format!("{}    └── In:\n", prefix));
        result.push_str(&body.print_tree_helper(&format!("{}        ", prefix), ""));
      },
      Expression::Proof { name, generics, signals, constraints } => {
        result.push_str(&format!("{}└── Proof({})\n", prefix, name));
        
        if !generics.is_empty() {
          result.push_str(&format!("{}    ├── Generics:\n", prefix));
          for (i, generic) in generics.iter().enumerate() {
            let is_last = i == generics.len() - 1;
            result.push_str(&format!("{}    │   {}── {}{}\n", 
              prefix,
              if is_last { "└" } else { "├" },
              generic.name,
              if let Some(bound) = &generic.bound {
                format!(": {}", bound)
              } else {
                String::new()
              }
            ));
          }
        }
        
        if !signals.is_empty() {
          result.push_str(&format!("{}    ├── Signals:\n", prefix));
          for (i, signal) in signals.iter().enumerate() {
            let is_last = i == signals.len() - 1;
            result.push_str(&format!("{}    │   {}── {} {}: {}\n", 
              prefix,
              if is_last { "└" } else { "├" },
              signal.visibility,
              signal.name,
              signal.typ));
          }
        }
        
        if !constraints.is_empty() {
          result.push_str(&format!("{}    └── Constraints:\n", prefix));
          for (i, constraint) in constraints.iter().enumerate() {
            let is_last = i == constraints.len() - 1;
            match constraint {
              Constraint::Assert(expr) => {
                result.push_str(&format!("{}        {}── Assert:\n", prefix, if is_last { "└" } else { "├" }));
                result.push_str(&expr.print_tree_helper(&format!("{}        {}   ", prefix, if is_last { " " } else { "│" }), ""));
              },
              Constraint::Verify(expr) => {
                result.push_str(&format!("{}        {}── Verify:\n", prefix, if is_last { "└" } else { "├" }));
                result.push_str(&expr.print_tree_helper(&format!("{}        {}   ", prefix, if is_last { " " } else { "│" }), ""));
              },
              Constraint::Let(expr) => {
                result.push_str(&format!("{}        {}── Let Constraint:\n", prefix, if is_last { "└" } else { "├" }));
                result.push_str(&expr.print_tree_helper(&format!("{}        {}   ", prefix, if is_last { " " } else { "│" }), ""));
              },
              Constraint::Match(expr) => {
                result.push_str(&format!("{}        {}── Match Constraint:\n", prefix, if is_last { "└" } else { "├" }));
                result.push_str(&expr.print_tree_helper(&format!("{}        {}   ", prefix, if is_last { " " } else { "│" }), ""));
              },
            }
          }
        }
      },
      Expression::Component { .. } => {
        result.push_str(&format!("{}└── Component (printing not implemented)\n", prefix));
      },
    }
    result
  }
}
