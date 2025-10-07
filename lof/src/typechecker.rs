use crate::ast::{Expression, Operator, Pattern, Type};
use std::collections::{HashMap, HashSet};
use std::fmt;

pub struct TypeChecker {
    symbols: HashMap<String, Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    UndefinedType(String),
    TypeMismatch { expected: Type, found: Type },
    ArgumentCountMismatch { expected: usize, found: usize },
    PatternMismatch { expected: Type, found: Pattern },
    NonBooleanInAssert(Type),
    EmptyMatchExpression,
    DuplicatePatternVariable(String),
    InvalidExpression
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            TypeError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            TypeError::UndefinedType(name) => write!(f, "Undefined type: {}", name),
            TypeError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
            TypeError::EmptyMatchExpression => write!(f, "Match expression must have at least one pattern"),
            TypeError::DuplicatePatternVariable(name) => write!(f, "Variable '{}' is bound multiple times in the same pattern", name),
            TypeError::InvalidExpression => write!(f, "Invalid expression"),
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
        }
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbols: HashMap::new(),
        }
    }

    fn read_variable(&self, name: &str) -> Result<Type, TypeError> {
        let var_type = self.symbols.get(name)
            .cloned()
            .ok_or_else(|| TypeError::UndefinedVariable(name.to_string()))?;
        
        Ok(var_type)
    }

    fn check_pattern_compatibility(&self, pattern: &Pattern, typ: &Type) -> Result<(), TypeError> {
        match (pattern, typ) {
            (Pattern::Variable(_), _) | (Pattern::Wildcard, _) => Ok(()),
            (Pattern::Literal(_), Type::Field) => Ok(()),
            (Pattern::Tuple(patterns), Type::Tuple(types)) => {
                if patterns.len() != types.len() {
                    return Err(TypeError::PatternMismatch { expected: typ.clone(), found: pattern.clone() });
                }
                for (p, t) in patterns.iter().zip(types.iter()) {
                    self.check_pattern_compatibility(p, t)?;
                }
                Ok(())
            }
            _ => Err(TypeError::PatternMismatch { expected: typ.clone(), found: pattern.clone() }),
        }
    }
    
    fn check_pattern_duplicates(&self, pattern: &Pattern, bound_vars: &mut HashSet<String>) -> Result<(), TypeError> {
        match pattern {
            Pattern::Variable(name) => {
                if bound_vars.contains(name) {
                    return Err(TypeError::DuplicatePatternVariable(name.clone()));
                }
                bound_vars.insert(name.clone());
                Ok(())
            }
            Pattern::Tuple(patterns) => {
                for pat in patterns {
                    self.check_pattern_duplicates(pat, bound_vars)?;
                }
                Ok(())
            }
            Pattern::Wildcard | Pattern::Literal(_) => Ok(()),
            _ => Ok(()),
        }
    }
    
    fn types_compatible(&self, type1: &Type, type2: &Type) -> bool {
        type1 == type2
    }
    
    pub fn check_program(&mut self, program: &[Expression]) -> Result<(), TypeError> {
        for expr in program {
            if let Expression::FunctionDef { name, params, return_type, .. } = expr {
                let resolved_params = params
                    .iter()
                    .map(|p| self.resolve_type(&p.typ))
                    .collect::<Result<Vec<_>, _>>()?;
                let resolved_return_type = self.resolve_type(return_type)?;
                
                let function_type = if resolved_params.is_empty() {
                    resolved_return_type
                } else {
                    self.build_curried_function_type(resolved_params, resolved_return_type)
                };
                
                self.symbols.insert(name.clone(), function_type);
            }
        }
    
        for expr in program {
            self.check_expression(expr)?;
        }
        Ok(())
    }
    fn build_curried_function_type(&self, params: Vec<Type>, return_type: Type) -> Type {
        if params.is_empty() {
            return_type
        } else if params.len() == 1 {
            Type::Function {
                params: vec![params[0].clone()],
                return_type: Box::new(return_type),
            }
        } else {
            let mut result = return_type;
            for param in params.into_iter().rev() {
                result = Type::Function {
                    params: vec![param],
                    return_type: Box::new(result),
                };
            }
            result
        }
    }
    
    fn apply_function(&mut self, mut function_type: Type, arguments: &[Expression]) -> Result<Type, TypeError> {
        if arguments.is_empty() {
            return Ok(function_type);
        }
    
        for _ in arguments {
            match function_type {
                Type::Function { params, return_type } => {
                    if params.len() != 1 {
                        return Err(TypeError::ArgumentCountMismatch {
                            expected: 1,
                            found: params.len(),
                        });
                    }
                    
                    
                    function_type = *return_type;
                }
                _ => {
                    return Err(TypeError::ArgumentCountMismatch {
                        expected: 0,
                        found: 1,
                    });
                }
            }
        }
        
        Ok(function_type)
    }
    
    pub fn check_expression(&mut self, expr: &Expression) -> Result<Type, TypeError> {
        match expr {
            Expression::Number(_) => Ok(Type::Field),
            Expression::Variable(name) => {
                self.read_variable(name)
            }
            Expression::Let { pattern, value, body } => {
                let value_type = self.check_expression(value)?;
                let mut symbols_backup = HashMap::new();
                
                match pattern {
                    Pattern::Variable(var_name) => {
                        if let Some(old_type) = self.symbols.get(var_name) {
                            symbols_backup.insert(var_name.clone(), old_type.clone());
                        }
                        self.symbols.insert(var_name.clone(), value_type.clone());
                    }
                    _ => {
                        self.bind_pattern(pattern, &value_type)?;
                    }
                }
                
                let body_type = self.check_expression(body)?;
                
                match pattern {
                    Pattern::Variable(var_name) => {
                        if let Some(old_type) = symbols_backup.get(var_name) {
                            self.symbols.insert(var_name.clone(), old_type.clone());
                        } else {
                            self.symbols.remove(var_name);
                        }
                    }
                    _ => {
                    }
                }
                
                Ok(body_type)
            }
            Expression::BinaryOp { left, op, right } => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;
                
                
                self.check_operator(op, &left_type, &right_type)
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
                if cond_type != Type::Bool {
                    return Err(TypeError::NonBooleanInAssert(cond_type));
                }
                Ok(Type::Unit)
            }
            Expression::FunctionDef { name, body, params, return_type } => {
                let original_symbols = self.symbols.clone();

                for param in params.iter() {
                    let resolved_param_type = self.resolve_type(&param.typ)?;
                    self.symbols.insert(param.name.clone(), resolved_param_type);
                }
                
                let body_type = self.check_expression(body)?;
                self.symbols = original_symbols;
                
                let expected_return_type = self.resolve_type(return_type)?;
                
                if body_type != expected_return_type {
                    return Err(TypeError::TypeMismatch {
                        expected: expected_return_type,
                        found: body_type,
                    });
                }
                Ok(Type::Unit)
            }
            Expression::FunctionCall { function, arguments } => {
                let function_type = self.symbols.get(function)
                    .cloned()
                    .ok_or_else(|| TypeError::UndefinedFunction(function.clone()))?;
                
                self.apply_function(function_type, arguments)
            }
            Expression::Proof { signals, body, .. } => {
                for signal in signals {
                    let resolved_type = self.resolve_type(&signal.typ)?;
                    self.symbols.insert(signal.name.clone(), resolved_type);
                }
            
                let _body_type = self.check_expression(body)?;
            
            
                Ok(Type::Unit)
            }
            Expression::Block { statements, final_expr } => {
                for stmt in statements {
                    self.check_expression(stmt)?;
                }
                let result_type = if let Some(expr) = final_expr {
                    self.check_expression(expr)?
                } else {
                    Type::Unit
                };
                
                Ok(result_type)
            }
            Expression::Match { value, patterns } => {
                if patterns.is_empty() {
                    return Err(TypeError::EmptyMatchExpression);
                }
                
                let scrutinee_type = self.check_expression(value)?;
                
                let mut arm_types = Vec::new();
                
                for pattern_arm in patterns {
                    let original_symbols = self.symbols.clone();
                    
                    self.check_pattern_compatibility(&pattern_arm.pattern, &scrutinee_type)?;
                    self.bind_pattern(&pattern_arm.pattern, &scrutinee_type)?;
                    
                    let arm_type = self.check_expression(&pattern_arm.body)?;
                    arm_types.push(arm_type);
                    
                    self.symbols = original_symbols;
                }
                
                if let Some(first_type) = arm_types.first() {
                    for arm_type in arm_types.iter().skip(1) {
                        if !self.types_compatible(&first_type, arm_type) {
                            return Err(TypeError::TypeMismatch {
                                expected: first_type.clone(),
                                found: arm_type.clone(),
                            });
                        }
                    }
                    Ok(first_type.clone())
                } else {
                    Err(TypeError::EmptyMatchExpression)
                }
            }
            _ => todo!("Type checking for this expression is not yet implemented: {:?}", expr),
        }
    }

    fn resolve_type(&self, typ: &Type) -> Result<Type, TypeError> {
        match typ {
            Type::Identifier(name) => match name.as_str() {
                "field" => Ok(Type::Field),
                "bool" => Ok(Type::Bool),
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
        let mut bound_vars = HashSet::new();
        self.check_pattern_duplicates(pattern, &mut bound_vars)?;
        
        match (pattern, typ) {
            (Pattern::Variable(name), _) => {
                self.symbols.insert(name.clone(), typ.clone());
                Ok(())
            }
            (Pattern::Wildcard, _) | (Pattern::Literal(_), _) => Ok(()),
            (Pattern::Tuple(patterns), Type::Tuple(types)) => {
                for (p, t) in patterns.iter().zip(types.iter()) {
                    self.bind_pattern(p, t)?;
                }
                Ok(())
            }
            _ => Err(TypeError::PatternMismatch { expected: typ.clone(), found: pattern.clone() }),
        }
    }

    fn check_operator(&mut self, op: &Operator, left: &Type, right: &Type) -> Result<Type, TypeError> {
        match op {
            Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                let left_is_field = matches!(left, Type::Field);
                let right_is_field = matches!(right, Type::Field);
                
                if left_is_field && right_is_field {
                    Ok(Type::Field)
                } else {
                    Err(TypeError::TypeMismatch { 
                        expected: Type::Field, 
                        found: if !left_is_field { left.clone() } else { right.clone() } 
                    })
                }
            }
    
            Operator::Equal | Operator::NotEqual => {
                if left == right {
                    Ok(Type::Bool)
                } else {
                    Err(TypeError::TypeMismatch { expected: left.clone(), found: right.clone() })
                }
            }
    
            Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge => {
                let left_is_field = matches!(left, Type::Field);
                let right_is_field = matches!(right, Type::Field);
                
                if left_is_field && right_is_field {
                    Ok(Type::Bool)
                } else {
                    Err(TypeError::TypeMismatch { 
                        expected: Type::Field, 
                        found: if !left_is_field { left.clone() } else { right.clone() } 
                    })
                }
            }
            
            Operator::And | Operator::Or => {
                if *left == Type::Bool && *right == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(TypeError::TypeMismatch { expected: Type::Bool, found: if *left != Type::Bool { left.clone() } else { right.clone() } })
                }
            }
            
            Operator::Assert => {
                if self.types_compatible(left, right) {
                    Ok(Type::Bool)
                } else {
                    Err(TypeError::TypeMismatch { expected: left.clone(), found: right.clone() })
                }
            }
        }
    }
}