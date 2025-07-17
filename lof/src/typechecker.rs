use crate::ast::{self, Expression, Operator, Pattern, Type, LinearityKind};
use std::collections::HashMap;
use std::fmt;

pub struct TypeChecker {
    symbols: HashMap<String, Type>,
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
    InvalidDup(Type)
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            TypeError::VariableAlreadyConsumed(name) => write!(f, "Variable '{}' has already been consumed", name),
            TypeError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            TypeError::UndefinedType(name) => write!(f, "Undefined type: {}", name),
            TypeError::InvalidDup(typ) => write!(f, "Cannot dup type: {}", typ),
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
        }
    }

    fn read_variable(&self, name: &str) -> Result<Type, TypeError> {
        let var_type = self.symbols.get(name)
            .cloned()
            .ok_or_else(|| TypeError::UndefinedVariable(name.to_string()))?;
        
        match &var_type {
            Type::Field(LinearityKind::Consumed) | Type::Bool(LinearityKind::Consumed) => {
                Err(TypeError::VariableAlreadyConsumed(name.to_string()))
            }
            _ => Ok(var_type)
        }
    }
    
    fn consume_variable(&mut self, name: &str) -> Result<Type, TypeError> {
        let var_type = self.symbols.get(name)
            .cloned()
            .ok_or_else(|| TypeError::UndefinedVariable(name.to_string()))?;
        
        println!("DEBUG: Trying to consume '{}' with type {:?}", name, var_type);
        
        match &var_type {
            Type::Field(LinearityKind::Consumed) | Type::Bool(LinearityKind::Consumed) => {
                println!("DEBUG: Variable '{}' already consumed!", name);
                return Err(TypeError::VariableAlreadyConsumed(name.to_string()));
            }
            _ => {}
        }
        
        match &var_type {
            Type::Field(LinearityKind::Linear) => {
                println!("DEBUG: Consuming linear variable '{}'", name);
                self.symbols.insert(name.to_string(), Type::Field(LinearityKind::Consumed));
            }
            Type::Bool(LinearityKind::Linear) => {
                println!("DEBUG: Consuming linear bool variable '{}'", name);
                self.symbols.insert(name.to_string(), Type::Bool(LinearityKind::Consumed));
            }
            _ => {
                println!("DEBUG: Variable '{}' is copyable, not consuming", name);
            }
        }
        
        Ok(var_type)
    }
    
    fn access_variable(&mut self, name: &str, consume: bool) -> Result<Type, TypeError> {
        if consume {
            self.consume_variable(name)
        } else {
            self.read_variable(name)
        }
    }
    
    fn consume_variables_in_expression(&mut self, expr: &Expression) -> Result<(), TypeError> {
        match expr {
            Expression::Variable(name) => {
                self.consume_variable(name)?;
                Ok(())
            }
            Expression::BinaryOp { left, right, .. } => {
                self.consume_variables_in_expression(left)?;
                self.consume_variables_in_expression(right)?;
                Ok(())
            }
            Expression::Tuple(elements) => {
                for elem in elements {
                    self.consume_variables_in_expression(elem)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
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
    
        for arg_expr in arguments {
            println!("DEBUG: apply_function processing argument: {:?}", arg_expr);
            
            match function_type {
                Type::Function { params, return_type } => {
                    if params.len() != 1 {
                        return Err(TypeError::ArgumentCountMismatch {
                            expected: 1,
                            found: params.len(),
                        });
                    }
                    
                    let arg_type = self.check_expression(arg_expr)?;
                    println!("DEBUG: Argument type checked: {:?}", arg_type);
                    
                    if let Expression::Variable(var_name) = arg_expr {
                        println!("DEBUG: About to consume variable: {}", var_name);
                        println!("DEBUG: Symbols before consumption: {:?}", self.symbols);
                        self.consume_variable(var_name)?;
                        println!("DEBUG: Symbols after consumption: {:?}", self.symbols);
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
            Expression::Number(_) => Ok(Type::Field(LinearityKind::Copyable)),
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
                self.consume_variables_in_expression(condition)?;
                if cond_type != Type::Bool(LinearityKind::Linear) {
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
                
                let return_types_compatible = match (&body_type, &expected_return_type) {
                    (Type::Field(LinearityKind::Copyable), Type::Field(LinearityKind::Linear)) => true,
                    (Type::Field(LinearityKind::Linear), Type::Field(LinearityKind::Linear)) => true,
                    _ => body_type == expected_return_type,
                };
                
                if !return_types_compatible {
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
            Expression::Dup(expr) => {
                let arg_type = self.check_expression(expr)?;
                
                match arg_type {
                    Type::Field(LinearityKind::Linear) => Ok(Type::Field(LinearityKind::Copyable)),
                    Type::Bool(LinearityKind::Linear) => Ok(Type::Bool(LinearityKind::Copyable)),
                    Type::Field(LinearityKind::Copyable) | Type::Bool(LinearityKind::Copyable) => {
                        Err(TypeError::InvalidDup(arg_type))
                    }
                    Type::Field(LinearityKind::Consumed) | Type::Bool(LinearityKind::Consumed) => {
                        Err(TypeError::VariableAlreadyConsumed("argument to dup".to_string()))
                    }
                    _ => {
                        Err(TypeError::TypeMismatch {
                            expected: Type::Field(LinearityKind::Linear),
                            found: arg_type,
                        })
                    }
                }
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
        match op {
            Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                if let Some(name) = left_name {
                    self.consume_variable(name)?;
                }
                if let Some(name) = right_name {
                    self.consume_variable(name)?;
                }
            }
            Operator::Equal | Operator::NotEqual | Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge => {
                // Comparison oprations READ their operands (don't consume)
                // Already read in check_expression, so nothing to do here
            }
            Operator::And | Operator::Or => {
                if let Some(name) = left_name {
                    self.consume_variable(name)?;
                }
                if let Some(name) = right_name {
                    self.consume_variable(name)?;
                }
            }
            Operator::Assert => {
                if let Some(name) = left_name {
                    self.consume_variable(name)?;
                }
                if let Some(name) = right_name {
                    self.consume_variable(name)?;
                }
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
                let types_compatible = match (left, right) {
                    (Type::Field(LinearityKind::Linear), Type::Field(LinearityKind::Linear)) => true,
                    (Type::Field(LinearityKind::Linear), Type::Field(LinearityKind::Copyable)) => true,
                    (Type::Field(LinearityKind::Copyable), Type::Field(LinearityKind::Linear)) => true,
                    (Type::Field(LinearityKind::Copyable), Type::Field(LinearityKind::Copyable)) => true,
                    _ => left == right,
                };
                
                if types_compatible {
                    Ok(Type::Bool(LinearityKind::Linear))
                } else {
                    Err(TypeError::TypeMismatch { expected: left.clone(), found: right.clone() })
                }
            }
    
            Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge => {
                let left_is_field = matches!(left, Type::Field(LinearityKind::Linear) | Type::Field(LinearityKind::Copyable));
                let right_is_field = matches!(right, Type::Field(LinearityKind::Linear) | Type::Field(LinearityKind::Copyable));
                
                if left_is_field && right_is_field {
                    Ok(Type::Bool(LinearityKind::Linear))
                } else {
                    Err(TypeError::TypeMismatch { 
                        expected: Type::Field(LinearityKind::Linear), 
                        found: if !left_is_field { left.clone() } else { right.clone() } 
                    })
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