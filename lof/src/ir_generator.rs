use crate::ast::{Expression, Operator, Parameter, Pattern, Type, Visibility};
use crate::ir::{bigint_to_ir_constant, IRCircuit, IRExpr, IRInstruction, IRType};
use num_bigint::BigInt;
use serde_json;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
};
use tracing::debug;

#[derive(Debug)]
pub enum IRGenError {
    UnsupportedExpression(String),
    UnknownVariable(String),
    InvalidPattern(String),
    TypeError(String),
}

pub struct IRGenerator {
    instructions: Vec<IRInstruction>,
    function_defs: HashMap<String, (Vec<Parameter>, Expression)>,
    component_defs: HashMap<String, (Vec<Parameter>, Expression)>,
    variable_substitutions: HashMap<String, IRExpr>,
}

impl IRGenerator {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            function_defs: HashMap::new(),
            component_defs: HashMap::new(),
            variable_substitutions: HashMap::new(),
        }
    }

    pub fn register_function(&mut self, name: String, params: Vec<Parameter>, body: Expression) {
        self.function_defs.insert(name, (params, body));
    }

    pub fn register_component(&mut self, name: String, params: Vec<Parameter>, body: Expression) {
        self.component_defs.insert(name, (params, body));
    }

    pub fn convert_proof(&mut self, expr: &Expression) -> Result<IRCircuit, IRGenError> {
        match expr {
            Expression::Proof {
                name,
                signals,
                body,
                ..
            } => {
                debug!("Converting proof '{}' to IR", name);

                self.instructions.clear();
                self.variable_substitutions.clear();

                let mut pub_inputs = Vec::new();
                let mut witnesses = Vec::new();

                for signal in signals {
                    let ir_type = self.convert_type(&signal.typ)?;

                    match signal.visibility {
                        Visibility::Input => {
                            self.flatten_signal_to_inputs(&signal.name, &ir_type, &mut pub_inputs);
                        }
                        Visibility::Witness => {
                            witnesses.push((signal.name.clone(), ir_type));
                        }
                    }
                }

                self.convert_expression_to_ir(body)?;

                let circuit = IRCircuit {
                    name: name.clone(),
                    pub_inputs,
                    witnesses,
                    outputs: Vec::new(),
                    instructions: self.instructions.clone(),
                    functions: HashMap::new(),
                };

                self.instructions.clear();
                self.variable_substitutions.clear();

                Ok(circuit)
            }
            _ => Err(IRGenError::UnsupportedExpression(
                "Expected proof expression".to_string(),
            )),
        }
    }

    fn flatten_signal_to_inputs(
        &self,
        name: &str,
        typ: &IRType,
        inputs: &mut Vec<(String, IRType)>,
    ) {
        match typ {
            IRType::Array { element_type, size } => {
                for i in 0..*size {
                    let indexed_name = format!("{}[{}]", name, i);
                    inputs.push((indexed_name, (**element_type).clone()));
                }
            }
            IRType::Tuple(field_types) => {
                for (i, field_type) in field_types.iter().enumerate() {
                    let component_name = format!("{}_{}", name, i);
                    inputs.push((component_name, field_type.clone()));
                }
            }
            _ => {
                // Scalar type
                inputs.push((name.to_string(), typ.clone()));
            }
        }
    }

    fn convert_type(&self, typ: &Type) -> Result<IRType, IRGenError> {
        match typ {
            Type::Field { .. } => Ok(IRType::Field),
            Type::Bool { .. } => Ok(IRType::Bool),
            Type::Array { element_type, size } => self.convert_array_type(element_type, *size),
            Type::Tuple(types) => self.convert_tuple_type(types),
            Type::Refined(base_type, _predicate) => self.convert_type(base_type),
            _ => Err(IRGenError::TypeError(format!(
                "Unsupported type in IR: {:?}",
                typ
            ))),
        }
    }

    fn convert_array_type(&self, element_type: &Type, size: usize) -> Result<IRType, IRGenError> {
        Ok(IRType::Array {
            element_type: Box::new(self.convert_type(element_type)?),
            size,
        })
    }

    fn convert_tuple_type(&self, types: &[Type]) -> Result<IRType, IRGenError> {
        let ir_types: Result<Vec<_>, _> = types.iter().map(|t| self.convert_type(t)).collect();
        Ok(IRType::Tuple(ir_types?))
    }

    fn convert_expression_to_ir(
        &mut self,
        expr: &Expression,
    ) -> Result<Option<IRExpr>, IRGenError> {
        match expr {
            Expression::Number(n) => {
                let bigint = BigInt::from(*n);
                Ok(Some(IRExpr::Constant(bigint_to_ir_constant(&bigint))))
            }

            Expression::Variable(name) => {
                if let Some(subst) = self.variable_substitutions.get(name) {
                    Ok(Some(subst.clone()))
                } else {
                    Ok(Some(IRExpr::Variable(name.clone())))
                }
            }

            Expression::BinaryOp { left, op, right } => {
                let left_expr = self.convert_expression_to_ir(left)?.ok_or_else(|| {
                    IRGenError::UnsupportedExpression("Empty left expr".to_string())
                })?;
                let right_expr = self.convert_expression_to_ir(right)?.ok_or_else(|| {
                    IRGenError::UnsupportedExpression("Empty right expr".to_string())
                })?;

                let result_expr = match op {
                    Operator::Add => IRExpr::Add(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Sub => IRExpr::Sub(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Mul => IRExpr::Mul(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Div => IRExpr::Div(Box::new(left_expr), Box::new(right_expr)),

                    Operator::Lt => IRExpr::Lt(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Gt => IRExpr::Gt(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Le => IRExpr::Le(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Ge => IRExpr::Ge(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Equal => IRExpr::Equal(Box::new(left_expr), Box::new(right_expr)),
                    Operator::NotEqual => {
                        IRExpr::NotEqual(Box::new(left_expr), Box::new(right_expr))
                    }

                    Operator::And => IRExpr::And(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Or => IRExpr::Or(Box::new(left_expr), Box::new(right_expr)),
                    Operator::Not => IRExpr::Not(Box::new(right_expr)), // Unary op

                    Operator::Assert => {
                        self.instructions.push(IRInstruction::Constrain {
                            left: left_expr.clone(),
                            right: right_expr.clone(),
                        });
                        IRExpr::Equal(Box::new(left_expr), Box::new(right_expr))
                    }
                };

                Ok(Some(result_expr))
            }

            Expression::Let {
                pattern,
                value,
                body,
            } => {
                let value_expr = self.convert_expression_to_ir(value)?;

                match value_expr {
                    Some(expr) => {
                        self.bind_pattern(pattern, expr)?;
                    }
                    None => {
                        if !matches!(pattern, Pattern::Wildcard) {
                            return Err(IRGenError::UnsupportedExpression(
                                "Let binding produced no value".to_string(),
                            ));
                        }
                    }
                }

                self.convert_expression_to_ir(body)
            }

            Expression::Assert(condition) => match self.convert_expression_to_ir(condition)? {
                Some(cond_expr) => {
                    self.instructions.push(IRInstruction::Assert {
                        condition: cond_expr,
                    });
                    Ok(None)
                }
                None => Ok(None),
            },

            Expression::Block {
                statements,
                final_expr,
            } => {
                for stmt in statements {
                    self.convert_expression_to_ir(stmt)?;
                }
                if let Some(expr) = final_expr {
                    self.convert_expression_to_ir(expr)
                } else {
                    Ok(None)
                }
            }

            Expression::Tuple(elements) => {
                let mut evaluated_elements = Vec::with_capacity(elements.len());
                for elem in elements {
                    match self.convert_expression_to_ir(elem)? {
                        Some(expr) => evaluated_elements.push(expr),
                        None => {
                            return Err(IRGenError::UnsupportedExpression(
                                "Tuple elements must produce values".to_string(),
                            ))
                        }
                    }
                }

                match evaluated_elements.len() {
                    0 => Ok(Some(Self::ir_constant(0))),
                    1 => Ok(Some(evaluated_elements.remove(0))),
                    _ => Err(IRGenError::UnsupportedExpression(
                        "Tuple expressions with more than one element are not yet supported in IR generation"
                            .to_string(),
                    )),
                }
            }

            Expression::ArrayIndex { array, index } => {
                let array_name = match array.as_ref() {
                    Expression::Variable(name) => name.clone(),
                    _ => {
                        return Err(IRGenError::UnsupportedExpression(
                            "Array indexing only supported for variables".to_string(),
                        ))
                    }
                };

                let index_value = match index.as_ref() {
                    Expression::Number(n) => *n as usize,
                    _ => {
                        return Err(IRGenError::UnsupportedExpression(
                            "Only constant array indices supported".to_string(),
                        ))
                    }
                };

                Ok(Some(IRExpr::ArrayIndex {
                    array: array_name,
                    index: index_value,
                }))
            }

            Expression::FunctionCall {
                function,
                arguments,
            } => {
                let def = self
                    .function_defs
                    .get(function)
                    .cloned()
                    .or_else(|| self.component_defs.get(function).cloned());

                if let Some((params, body)) = def {
                    let saved_substitutions = self.variable_substitutions.clone();

                    for (param, arg) in params.iter().zip(arguments.iter()) {
                        let arg_expr = self.convert_expression_to_ir(arg)?.ok_or_else(|| {
                            IRGenError::UnsupportedExpression("Empty function arg".to_string())
                        })?;
                        self.variable_substitutions
                            .insert(param.name.clone(), arg_expr);
                    }

                    let result = self.convert_expression_to_ir(&body)?;

                    self.variable_substitutions = saved_substitutions;

                    Ok(result)
                } else {
                    Err(IRGenError::UnknownVariable(format!(
                        "Function '{}' not found",
                        function
                    )))
                }
            }

            Expression::Match { value, patterns } => {
                if patterns.is_empty() {
                    return Err(IRGenError::UnsupportedExpression(
                        "Match expression with no patterns".to_string(),
                    ));
                }

                let match_value = self.convert_expression_to_ir(value)?.ok_or_else(|| {
                    IRGenError::UnsupportedExpression(
                        "Match value did not produce an expression".to_string(),
                    )
                })?;

                let mut remaining_selector = Self::ir_constant(1);
                let mut accumulated: Option<IRExpr> = None;

                for (idx, pattern_arm) in patterns.iter().enumerate() {
                    let (branch_guard, bindings) =
                        self.match_pattern_condition(&pattern_arm.pattern, &match_value)?;

                    let selector =
                        IRExpr::Mul(Box::new(remaining_selector.clone()), Box::new(branch_guard));

                    let saved_substitutions = self.variable_substitutions.clone();
                    for (name, binding_expr) in bindings {
                        self.variable_substitutions.insert(name, binding_expr);
                    }

                    let branch_value = self
                        .convert_expression_to_ir(&pattern_arm.body)?
                        .ok_or_else(|| {
                            IRGenError::UnsupportedExpression(format!(
                                "Match arm {} did not produce a value",
                                idx
                            ))
                        })?;

                    self.variable_substitutions = saved_substitutions;

                    let weighted_branch =
                        IRExpr::Mul(Box::new(selector.clone()), Box::new(branch_value));

                    accumulated = Some(match accumulated {
                        Some(current) => IRExpr::Add(Box::new(current), Box::new(weighted_branch)),
                        None => weighted_branch,
                    });

                    if idx < patterns.len() - 1 {
                        remaining_selector =
                            IRExpr::Sub(Box::new(remaining_selector), Box::new(selector));
                    }
                }

                if let Some(result_expr) = accumulated {
                    Ok(Some(result_expr))
                } else {
                    Err(IRGenError::UnsupportedExpression(
                        "Match expression produced no result".to_string(),
                    ))
                }
            }

            Expression::ArrayLiteral(_elements) => Err(IRGenError::UnsupportedExpression(
                "Array literals are not yet supported in IR generation".to_string(),
            )),

            _ => Err(IRGenError::UnsupportedExpression(format!(
                "Unsupported expression in IR generation: {:?}",
                expr
            ))),
        }
    }

    fn ir_constant(value: i64) -> IRExpr {
        let bigint = BigInt::from(value);
        IRExpr::Constant(bigint_to_ir_constant(&bigint))
    }

    fn match_pattern_condition(
        &self,
        pattern: &Pattern,
        match_value: &IRExpr,
    ) -> Result<(IRExpr, Vec<(String, IRExpr)>), IRGenError> {
        match pattern {
            Pattern::Literal(lit) => {
                let literal = IRExpr::Constant(bigint_to_ir_constant(&BigInt::from(*lit)));
                Ok((
                    IRExpr::Equal(Box::new(match_value.clone()), Box::new(literal)),
                    Vec::new(),
                ))
            }
            Pattern::Wildcard => Ok((Self::ir_constant(1), Vec::new())),
            Pattern::Variable(name) => Ok((
                Self::ir_constant(1),
                vec![(name.clone(), match_value.clone())],
            )),
            Pattern::Tuple(_) | Pattern::Constructor(_, _) => {
                Err(IRGenError::UnsupportedExpression(
                    "Tuple and constructor patterns are not yet supported in IR generation"
                        .to_string(),
                ))
            }
        }
    }

    /// Bind a pattern to an expression
    fn bind_pattern(&mut self, pattern: &Pattern, expr: IRExpr) -> Result<(), IRGenError> {
        match pattern {
            Pattern::Variable(name) => {
                self.instructions.push(IRInstruction::Assign {
                    target: name.clone(),
                    expr: expr.clone(),
                });

                self.variable_substitutions.insert(name.clone(), expr);

                Ok(())
            }

            Pattern::Wildcard => Ok(()),

            Pattern::Tuple(patterns) => {
                if let IRExpr::Variable(tuple_name) = expr {
                    for (i, sub_pattern) in patterns.iter().enumerate() {
                        let component_expr = IRExpr::TupleField {
                            tuple: tuple_name.clone(),
                            index: i,
                        };
                        self.bind_pattern(sub_pattern, component_expr)?;
                    }
                    Ok(())
                } else {
                    Err(IRGenError::InvalidPattern(
                        "Tuple pattern requires variable".to_string(),
                    ))
                }
            }

            Pattern::Literal(lit) => {
                let lit_expr = IRExpr::Constant(bigint_to_ir_constant(&BigInt::from(*lit)));
                self.instructions.push(IRInstruction::Constrain {
                    left: expr,
                    right: lit_expr,
                });
                Ok(())
            }

            Pattern::Constructor(_, _) => Err(IRGenError::UnsupportedExpression(
                "Constructor patterns are not yet supported in IR generation".to_string(),
            )),
        }
    }
}

impl IRGenerator {
    pub fn write_ir_file(
        &self,
        path: &std::path::Path,
        circuit: &IRCircuit,
    ) -> std::io::Result<u64> {
        let json = serde_json::to_vec_pretty(circuit).map_err(std::io::Error::other)?;
        let mut writer = BufWriter::new(File::create(path)?);
        writer.write_all(&json)?;
        Ok(json.len() as u64)
    }
}

impl Default for IRGenerator {
    fn default() -> Self {
        Self::new()
    }
}
