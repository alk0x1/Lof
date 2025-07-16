use crate::ast::{self, Expression, Operator, Pattern, Type, LinearityKind};
use std::collections::HashMap;
use std::fmt;

pub struct TypeChecker {
    symbols: HashMap<String, Type>,
    functions: HashMap<String, (Vec<ast::Parameter>, Type)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    VariableAlreadyConsumed(String),
    UndefinedType(String),
    TypeMismatch { expected: Type, found: Type },
    ArgumentCountMismatch { expected: usize, found: usize },
    PatternMismatch { expected: Type, found: Pattern },
    NonBooleanInAssert(Type),
    VariableAlreadyConsumedAt { 
        name: String, 
        first_use_line: Option<usize>,
        second_use_line: Option<usize> 
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            TypeError::VariableAlreadyConsumed(name) => write!(f, "Variable '{}' has already been consumed", name),
            TypeError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            TypeError::UndefinedType(name) => write!(f, "Undefined type: {}", name),
            TypeError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            TypeError::ArgumentCountMismatch { expected, found } => {
                write!(
                    f,
                    "Argument count mismatch: expected {}, found {}",
                    expected, found
                )
            }
            TypeError::PatternMismatch { expected, found } => {
                write!(
                    f,
                    "Pattern mismatch: pattern {:?} does not match type {}",
                    found, expected
                )
            }
            TypeError::NonBooleanInAssert(found) => {
                write!(f, "Assertion requires a boolean condition, found {}", found)
            }
            TypeError::VariableAlreadyConsumedAt { name, first_use_line, second_use_line } => {
                match (first_use_line, second_use_line) {
                    (Some(first), Some(second)) => write!(f, "Variable '{}' already consumed at line {}, cannot use again at line {}", name, first, second),
                    (Some(first), None) => write!(f, "Variable '{}' already consumed at line {}, cannot use again", name, first),
                    (None, Some(second)) => write!(f, "Variable '{}' already consumed, attempted reuse at line {}", name, second),
                    (None, None) => write!(f, "Variable '{}' has already been consumed", name),
                }
            }
            
        }
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbols: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn check_program(&mut self, program: &[Expression]) -> Result<(), TypeError> {
        for expr in program {
            if let Expression::FunctionDef { name, params, return_type, .. } = expr {
                let resolved_params = params
                    .iter()
                    .map(|p| self.resolve_type(&p.typ).map(|t| ast::Parameter { name: p.name.clone(), typ: t }))
                    .collect::<Result<Vec<_>, _>>()?;
                let resolved_return_type = self.resolve_type(return_type)?;
                self.functions.insert(name.clone(), (resolved_params, resolved_return_type));
            }
        }

        for expr in program {
            self.check_expression(expr)?;
        }
        Ok(())
    }

    /// Recursively checks an expression and returns its type.
    pub fn check_expression(&mut self, expr: &Expression) -> Result<Type, TypeError> {
        match expr {
            Expression::Number(_) => Ok(Type::Field(LinearityKind::Copyable)),
            Expression::Variable(name) => {
                println!("Checking variable {}: {:?}", name, self.symbols.get(name));
                let mut var_type = self.symbols.get(name)
                .cloned()
                .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))?;
            
            match &mut var_type {
                Type::Field(LinearityKind::Consumed) | Type::Bool(LinearityKind::Consumed) => {
                    return Err(TypeError::VariableAlreadyConsumed(name.clone()));
                }
                _ => {}
            }
            
            Ok(var_type)
            }
            Expression::Let { pattern, value, body } => {
                let value_type = self.check_expression(value)?;
                
                let symbols_after_value = self.symbols.clone();
                
                self.bind_pattern(pattern, &value_type)?;
                
                let body_type = self.check_expression(body)?;
                
                self.symbols = symbols_after_value;
                
                Ok(body_type)
            }
            Expression::BinaryOp { left, op, right } => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;
                
                let left_name = if let Expression::Variable(name) = left.as_ref() { Some(name) } else { None };
                let right_name = if let Expression::Variable(name) = right.as_ref() { Some(name) } else { None };
                
                self.check_operator(op, &left_type, &right_type, left_name, right_name)
            }
            Expression::Tuple(elements) => {
                let types = elements
                    .iter()
                    .map(|e| self.check_expression(e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Type::Tuple(types))
            }
            Expression::Assert(condition) => {
                let cond_type = self.check_expression(condition)?;
                if cond_type != Type::Bool(LinearityKind::Linear) {
                    return Err(TypeError::NonBooleanInAssert(cond_type));
                }
                Ok(Type::Unit)
            }
            Expression::FunctionDef { name, body, .. } => {
                let original_symbols = self.symbols.clone();
                let (resolved_params, resolved_return_type) = self.functions.get(name).unwrap().clone();
                for param in &resolved_params {
                    self.symbols.insert(param.name.clone(), param.typ.clone());
                }
                let body_type = self.check_expression(body)?;
                self.symbols = original_symbols;
                if body_type != resolved_return_type {
                    return Err(TypeError::TypeMismatch {
                        expected: resolved_return_type,
                        found: body_type,
                    });
                }
                Ok(Type::Unit)
            }
            Expression::FunctionCall { function, arguments } => {
                let (expected_params, return_type) = self.functions.get(function)
                    .cloned()
                    .ok_or_else(|| TypeError::UndefinedFunction(function.clone()))?;
                if arguments.len() != expected_params.len() {
                    return Err(TypeError::ArgumentCountMismatch {
                        expected: expected_params.len(),
                        found: arguments.len(),
                    });
                }
                for (arg_expr, param) in arguments.iter().zip(expected_params.iter()) {
                    let arg_type = self.check_expression(arg_expr)?;
                    if arg_type != param.typ {
                        return Err(TypeError::TypeMismatch {
                            expected: param.typ.clone(),
                            found: arg_type,
                        });
                    }
                }
                Ok(return_type)
            }
            Expression::Proof { signals, body, .. } => {
                // Bind all input/witness signals
                for signal in signals {
                    let resolved_type = self.resolve_type(&signal.typ)?;
                    self.symbols.insert(signal.name.clone(), resolved_type);
                }
                
                let _body_type = self.check_expression(body)?;
                
                Ok(Type::Unit)
            }
            Expression::Block { statements, final_expr } => {
                let original_symbols = self.symbols.clone();
                for stmt in statements {
                    self.check_expression(stmt)?;
                }
                let result_type = if let Some(expr) = final_expr {
                    self.check_expression(expr)?
                } else {
                    Type::Unit
                };
                self.symbols = original_symbols;
                Ok(result_type)
            }
            _ => todo!("Type checking for this expression is not yet implemented: {:?}", expr),
        }
    }

    fn resolve_type(&self, typ: &Type) -> Result<Type, TypeError> {
        match typ {
            Type::Identifier(name) => match name.as_str() {
                "field" => Ok(Type::Field(LinearityKind::Linear)),
                "bool" => Ok(Type::Bool(LinearityKind::Linear)),
                "unit" => Ok(Type::Unit),
                _ => Err(TypeError::UndefinedType(name.clone())),
            },
            Type::Tuple(elements) => {
                let resolved_elements = elements
                    .iter()
                    .map(|t| self.resolve_type(t))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Type::Tuple(resolved_elements))
            }
            _ => Ok(typ.clone()),
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern, typ: &Type) -> Result<(), TypeError> {
        match (pattern, typ) {
            (Pattern::Variable(name), _) => {
                self.symbols.insert(name.clone(), typ.clone());
                Ok(())
            }
            (Pattern::Wildcard, _) => Ok(()),
            (Pattern::Tuple(patterns), Type::Tuple(types)) => {
                if patterns.len() != types.len() {
                    return Err(TypeError::PatternMismatch { expected: typ.clone(), found: pattern.clone() });
                }
                for (p, t) in patterns.iter().zip(types.iter()) {
                    self.bind_pattern(p, t)?;
                }
                Ok(())
            }
            _ => Err(TypeError::PatternMismatch { expected: typ.clone(), found: pattern.clone() }),
        }
    }

    fn check_operator(&mut self, op: &Operator, left: &Type, right: &Type, left_name: Option<&String>, right_name: Option<&String>) -> Result<Type, TypeError> {
        if let Some(name) = left_name {
            if matches!(left, Type::Field(LinearityKind::Consumed) | Type::Bool(LinearityKind::Consumed)) {
                return Err(TypeError::VariableAlreadyConsumed(name.clone()));
            }
        }
        if let Some(name) = right_name {
            if matches!(right, Type::Field(LinearityKind::Consumed) | Type::Bool(LinearityKind::Consumed)) {
                return Err(TypeError::VariableAlreadyConsumed(name.clone()));
            }
        }
        if let Some(name) = left_name {
            if matches!(left, Type::Field(LinearityKind::Linear) | Type::Bool(LinearityKind::Linear)) {
                let consumed_type = match left {
                    Type::Field(_) => Type::Field(LinearityKind::Consumed),
                    Type::Bool(_) => Type::Bool(LinearityKind::Consumed),
                    _ => unreachable!()
                };
                self.symbols.insert(name.clone(), consumed_type);
            }
        }
        
        match op {
            Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                let left_is_field = matches!(left, Type::Field(LinearityKind::Linear) | Type::Field(LinearityKind::Copyable));
                let right_is_field = matches!(right, Type::Field(LinearityKind::Linear) | Type::Field(LinearityKind::Copyable));

                if left_is_field && right_is_field {
                    Ok(Type::Field(LinearityKind::Linear))
                } else {
                    Err(TypeError::TypeMismatch { 
                        expected: Type::Field(LinearityKind::Linear), 
                        found: if !left_is_field { left.clone() } else { right.clone() } 
                    })
                }
            }
            Operator::Equal | Operator::NotEqual => {
                if left != right {
                    return Err(TypeError::TypeMismatch { expected: left.clone(), found: right.clone() });
                }
                Ok(Type::Bool(LinearityKind::Linear))
            }
            Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge => {
                if *left == Type::Field(LinearityKind::Linear) && *right == Type::Field(LinearityKind::Linear) {
                    Ok(Type::Bool(LinearityKind::Linear))
                } else {
                    Err(TypeError::TypeMismatch { expected: Type::Field(LinearityKind::Linear), found: if *left != Type::Field(LinearityKind::Linear) { left.clone() } else { right.clone() } })
                }
            }
            Operator::And | Operator::Or => {
                if *left == Type::Bool(LinearityKind::Linear) && *right == Type::Bool(LinearityKind::Linear) {
                    Ok(Type::Bool(LinearityKind::Linear))
                } else {
                    Err(TypeError::TypeMismatch { expected: Type::Bool(LinearityKind::Linear), found: if *left != Type::Bool(LinearityKind::Linear) { left.clone() } else { right.clone() } })
                }
            }
            Operator::Assert => {
                if *left == *right {
                    Ok(Type::Unit)
                } else {
                    Err(TypeError::TypeMismatch { expected: left.clone(), found: right.clone() })
                }
            }
        }
    }
}