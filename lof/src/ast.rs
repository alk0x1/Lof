use std::fmt;

/// Tracks whether a field or bool value is constrained in the circuit
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ConstraintStatus {
    /// Witness declared but never used in a constraint
    Unconstrained,
    /// Used in at least one constraint (multiplication, assertion, match, etc.)
    Constrained,
}

/// Refinement types for preventing specific vulnerabilities
#[derive(Debug, Clone, PartialEq)]
pub enum Refinement {
    /// NonZero refinement - prevents division by zero
    NonZero,
    /// Range refinement - prevents field overflow and missing range checks
    Range { min: i64, max: i64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // Core types with constraint tracking
    Field {
        constraint: ConstraintStatus,
        refinement: Option<Refinement>,
    },
    Bool {
        constraint: ConstraintStatus,
    },

    // Other core types
    Bits(Box<Expression>),
    Array {
        element_type: Box<Type>,
        size: usize,
    },
    Nat,

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

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Field {
                constraint,
                refinement,
            } => {
                let constraint_str = match constraint {
                    ConstraintStatus::Constrained => "^constrained",
                    ConstraintStatus::Unconstrained => "^unconstrained",
                };
                match refinement {
                    Some(Refinement::NonZero) => write!(f, "NonZero<field{}>", constraint_str),
                    Some(Refinement::Range { min, max }) => {
                        write!(f, "Range<field{}, {}, {}>", constraint_str, min, max)
                    }
                    None => write!(f, "field{}", constraint_str),
                }
            }
            Self::Bool { constraint } => {
                let constraint_str = match constraint {
                    ConstraintStatus::Constrained => "^constrained",
                    ConstraintStatus::Unconstrained => "^unconstrained",
                };
                write!(f, "bool{}", constraint_str)
            }
            Self::Bits(size) => write!(f, "Bits<{:?}>", size),
            Self::Array { element_type, size } => write!(f, "Array<{}, {}>", element_type, size),
            Self::Nat => write!(f, "Nat"),
            Self::Custom(name) => write!(f, "{}", name),
            Self::GenericType(name) => write!(f, "{}", name),
            Self::Unit => write!(f, "()"),
            Self::Function {
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "Function<({}), {}>", params_str, return_type)
            }
            Self::Refined(base, expr) => write!(f, "Refined<{}, {:?}>", base, expr),
            Self::Identifier(name) => write!(f, "{}", name),
            Self::Tuple(types) => {
                let types_str = types
                    .iter()
                    .map(|t| format!("{}", t))
                    .collect::<Vec<_>>()
                    .join(", ");
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
    ArrayLiteral(Vec<Expression>),
    TypeAlias {
        name: String,
        typ: Type,
    },
    EnumDef {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Type>,
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
    Witness, // implicitly linear
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Variable(String),
    Tuple(Vec<Pattern>),
    Wildcard,
    Constructor(String, Vec<Pattern>),
    Literal(i64),
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
    And, // &&
    Or,  // ||
    Not, // !

    // Constraint
    Assert, // ===
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub bound: Option<Type>,
}
