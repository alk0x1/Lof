use crate::ast::{Expression, Operator, Parameter, Pattern, Type, Visibility};
use num_bigint::BigInt;
use std::fmt;
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek, Write},
    path::PathBuf,
};
use tracing::{debug, info, warn};

#[derive(Debug)]
pub enum R1CSError {
    UnsupportedOperation(String),
    InvalidFunction(String),
    InvalidArgument(String),
    NonQuadratic,
    InvalidExpression,
    FileError,
    TypeError(String),
    UnknownVariable(String),
}

#[derive(Debug, Clone)]
pub struct R1CSConstraint {
    pub a: LinearCombination,
    pub b: LinearCombination,
    pub c: LinearCombination,
}

#[derive(Debug, Clone)]
pub struct LinearCombination {
    pub terms: Vec<(String, BigInt)>,
}

#[derive(Debug, Clone)]
pub struct R1CSContext {
    pub variables: HashMap<String, Type>,
}

pub struct R1CSGenerator {
    pub constraints: Vec<R1CSConstraint>,
    pub temp_var_counter: usize,
    pub symbol_map: HashMap<String, usize>,
    pub variable_substitutions: HashMap<String, LinearCombination>,
    pub pub_inputs: Vec<String>,
    pub witnesses: Vec<String>,
    pub context: R1CSContext,
    pub function_defs: HashMap<String, (Vec<Parameter>, Expression)>,
    pub arrays: HashMap<String, Vec<String>>,
    pub boolean_vars: HashSet<String>,
}

impl Default for R1CSGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl R1CSGenerator {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            temp_var_counter: 0,
            symbol_map: HashMap::new(),
            variable_substitutions: HashMap::new(),
            pub_inputs: Vec::new(),
            witnesses: Vec::new(),
            context: R1CSContext {
                variables: HashMap::new(),
            },
            function_defs: HashMap::new(),
            arrays: HashMap::new(),
            boolean_vars: HashSet::new(),
        }
    }

    pub fn register_function(&mut self, name: String, params: Vec<Parameter>, body: Expression) {
        self.function_defs.insert(name, (params, body));
    }

    fn reset_for_new_proof(&mut self) {
        self.constraints.clear();
        self.temp_var_counter = 0;
        self.symbol_map.clear();
        self.variable_substitutions.clear();
        self.pub_inputs.clear();
        self.witnesses.clear();
        self.context.variables.clear();
        self.arrays.clear();
        self.boolean_vars.clear();
    }

    pub fn write_r1cs_file(&self, source_path: &std::path::Path) -> std::io::Result<u64> {
        let mut r1cs_path = source_path
            .parent()
            .ok_or_else(|| std::io::Error::other("Could not determine parent directory"))?
            .to_path_buf();

        let file_stem = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        r1cs_path.push(format!("{}.r1cs", file_stem));

        info!("Writing R1CS file to: {}", r1cs_path.display());

        let file = std::fs::File::create(&r1cs_path)?;
        let mut writer = std::io::BufWriter::new(file);

        writer.write_all(b"lof-r1cs")?;
        writer.write_all(&1u32.to_le_bytes())?;

        writer.write_all(&(self.pub_inputs.len() as u32).to_le_bytes())?;
        writer.write_all(&(self.witnesses.len() as u32).to_le_bytes())?;
        writer.write_all(&(self.constraints.len() as u32).to_le_bytes())?;

        for input in &self.pub_inputs {
            writer.write_all(&(input.len() as u32).to_le_bytes())?;
            writer.write_all(input.as_bytes())?;
        }

        for witness in &self.witnesses {
            writer.write_all(&(witness.len() as u32).to_le_bytes())?;
            writer.write_all(witness.as_bytes())?;
        }

        for constraint in &self.constraints {
            self.write_linear_combination(&mut writer, &constraint.a)?;
            self.write_linear_combination(&mut writer, &constraint.b)?;
            self.write_linear_combination(&mut writer, &constraint.c)?;
        }

        let metadata = std::fs::metadata(&r1cs_path)?;
        info!(
            "Successfully wrote {} bytes to {} ({} constraints)",
            metadata.len(),
            r1cs_path.display(),
            self.constraints.len()
        );

        Ok(metadata.len())
    }

    fn write_linear_combination<W: Write + Seek>(
        &self,
        writer: &mut W,
        lc: &LinearCombination,
    ) -> std::io::Result<()> {
        writer.write_all(&(lc.terms.len() as u32).to_le_bytes())?;

        for (var, coeff) in &lc.terms {
            let idx = self.get_variable_index(var);
            writer.write_all(&(idx as u32).to_le_bytes())?;

            let bytes = coeff.to_signed_bytes_le();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(&bytes)?;
        }

        Ok(())
    }

    fn new_temp_var(&mut self) -> String {
        let var = format!("t_{}", self.temp_var_counter);
        self.temp_var_counter += 1;
        var
    }

    fn enforce_boolean(&mut self, var: &str) {
        if self.boolean_vars.insert(var.to_string()) {
            self.constraints.push(R1CSConstraint {
                a: LinearCombination {
                    terms: vec![(var.to_string(), BigInt::from(1))],
                },
                b: LinearCombination {
                    terms: vec![
                        ("ONE".to_string(), BigInt::from(1)),
                        (var.to_string(), BigInt::from(-1)),
                    ],
                },
                c: LinearCombination { terms: vec![] },
            });
        }
    }

    pub fn convert_proof(&mut self, expr: &Expression) -> Result<(), R1CSError> {
        match expr {
            Expression::Proof {
                name,
                signals,
                body,
                ..
            } => {
                debug!("Converting proof '{}' to R1CS", name);

                self.reset_for_new_proof();

                for signal in signals {
                    match signal.visibility {
                        Visibility::Input => match &signal.typ {
                            Type::Array {
                                element_type: _,
                                size,
                            } => {
                                debug!(
                                    "Expanding array input '{}' into {} elements",
                                    signal.name, size
                                );
                                let mut element_vars = Vec::new();
                                for i in 0..*size {
                                    let indexed_name = format!("{}[{}]", signal.name, i);
                                    self.pub_inputs.push(indexed_name.clone());
                                    element_vars.push(indexed_name);
                                }
                                self.arrays.insert(signal.name.clone(), element_vars);
                                self.context
                                    .variables
                                    .insert(signal.name.clone(), signal.typ.clone());
                            }
                            Type::Tuple(field_types) => {
                                debug!(
                                    "Expanding tuple input '{}' into {} components",
                                    signal.name,
                                    field_types.len()
                                );
                                for i in 0..field_types.len() {
                                    let component_name = format!("{}_{}", signal.name, i);
                                    self.pub_inputs.push(component_name);
                                }
                                self.context
                                    .variables
                                    .insert(signal.name.clone(), signal.typ.clone());
                            }
                            _ => {
                                self.pub_inputs.push(signal.name.clone());
                                self.context
                                    .variables
                                    .insert(signal.name.clone(), signal.typ.clone());
                                if matches!(signal.typ, Type::Bool { .. }) {
                                    self.enforce_boolean(&signal.name);
                                }
                            }
                        },
                        Visibility::Witness => {
                            self.witnesses.push(signal.name.clone());
                            self.context
                                .variables
                                .insert(signal.name.clone(), signal.typ.clone());
                            if matches!(signal.typ, Type::Bool { .. }) {
                                self.enforce_boolean(&signal.name);
                            }
                        }
                    }
                }

                let _result = self.convert_to_linear_combination(body)?;

                debug!(
                    "Generated {} constraints for proof '{}'",
                    self.constraints.len(),
                    name
                );
                warn!(
                    "Witnesses list ({} total): {:?}",
                    self.witnesses.len(),
                    self.witnesses
                );
                Ok(())
            }
            _ => Err(R1CSError::InvalidArgument(
                "Expected proof expression".to_string(),
            )),
        }
    }

    fn convert_to_linear_combination(
        &mut self,
        expr: &Expression,
    ) -> Result<LinearCombination, R1CSError> {
        match expr {
            Expression::Variable(name) => {
                if let Some(subst) = self.variable_substitutions.get(name) {
                    return Ok(subst.clone());
                }

                Ok(LinearCombination {
                    terms: vec![(name.clone(), BigInt::from(1))],
                })
            }

            Expression::Number(n) => Ok(LinearCombination {
                terms: vec![("ONE".to_string(), BigInt::from(*n))],
            }),

            Expression::BinaryOp { left, op, right } => self.convert_binary_op(left, op, right),

            Expression::Assert(condition) => {
                let cond_lc = self.convert_to_linear_combination(condition)?;

                self.constraints.push(R1CSConstraint {
                    a: cond_lc.clone(),
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                });

                Ok(LinearCombination { terms: vec![] })
            }

            Expression::Let {
                pattern,
                value,
                body,
            } => self.convert_let_binding(pattern, value, body),

            Expression::Match { value, patterns } => self.convert_match_expression(value, patterns),

            Expression::Block {
                statements,
                final_expr,
            } => {
                let saved_variables = self.context.variables.clone();

                for stmt in statements {
                    self.convert_to_linear_combination(stmt)?;
                }

                let result = if let Some(expr) = final_expr {
                    self.convert_to_linear_combination(expr)?
                } else {
                    LinearCombination { terms: vec![] }
                };

                self.context.variables = saved_variables;

                Ok(result)
            }

            Expression::Tuple(elements) => {
                // we might need to flatten tuples or handle them specially
                // for now, just process each element
                for elem in elements {
                    self.convert_to_linear_combination(elem)?;
                }
                Ok(LinearCombination { terms: vec![] })
            }

            Expression::FunctionCall {
                function,
                arguments,
            } => self.convert_function_call(function, arguments),

            Expression::ArrayLiteral(_elements) => Err(R1CSError::UnsupportedOperation(
                "Array literals are not supported in R1CS generation yet".to_string(),
            )),

            Expression::ArrayIndex { array, index } => {
                // we only support constant indices for now
                let array_name = match array.as_ref() {
                    Expression::Variable(name) => name,
                    _ => {
                        warn!("Array indexing only supported for simple variables");
                        return Ok(LinearCombination { terms: vec![] });
                    }
                };

                let index_value = match index.as_ref() {
                    Expression::Number(n) => *n as usize,
                    _ => {
                        return Err(R1CSError::UnsupportedOperation(
                            "Only constant array indices are supported in R1CS generation"
                                .to_string(),
                        ))
                    }
                };

                if let Some(element_vars) = self.arrays.get(array_name) {
                    if index_value >= element_vars.len() {
                        return Err(R1CSError::UnsupportedOperation(format!(
                            "Array index {} out of bounds for array {} (length {})",
                            index_value,
                            array_name,
                            element_vars.len()
                        )));
                    }

                    let elem_var = &element_vars[index_value];
                    debug!(
                        "Array access {}[{}] -> {}",
                        array_name, index_value, elem_var
                    );

                    Ok(LinearCombination {
                        terms: vec![(elem_var.clone(), BigInt::from(1))],
                    })
                } else {
                    Err(R1CSError::UnsupportedOperation(format!(
                        "Array '{}' not found in array mapping",
                        array_name
                    )))
                }
            }

            Expression::TypeAlias { .. } | Expression::EnumDef { .. } => {
                Ok(LinearCombination { terms: vec![] })
            }

            _ => {
                warn!("Unsupported expression type in R1CS conversion: {:?}", expr);
                Ok(LinearCombination { terms: vec![] })
            }
        }
    }

    fn convert_binary_op(
        &mut self,
        left: &Expression,
        op: &Operator,
        right: &Expression,
    ) -> Result<LinearCombination, R1CSError> {
        match op {
            Operator::Add => {
                let mut left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;
                left_lc.add(&right_lc);
                Ok(left_lc)
            }

            Operator::Sub => {
                let mut left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;
                left_lc.add(&right_lc.negate());
                Ok(left_lc)
            }

            Operator::Mul => {
                // multiplication requires a new constraint: a * b = c
                let temp = self.new_temp_var();

                self.witnesses.push(temp.clone());

                let a = self.convert_to_linear_combination(left)?;
                let b = self.convert_to_linear_combination(right)?;

                let constraint = R1CSConstraint {
                    a,
                    b,
                    c: LinearCombination {
                        terms: vec![(temp.clone(), BigInt::from(1))],
                    },
                };
                warn!(
                    "PUSHING MULTIPLICATION CONSTRAINT #{}: {:?}",
                    self.constraints.len(),
                    constraint
                );
                self.constraints.push(constraint);

                Ok(LinearCombination {
                    terms: vec![(temp, BigInt::from(1))],
                })
            }

            Operator::Div => {
                // division: a / b = c  is encoded as  b * c = a
                // so we create a witness for c (the quotient) and add constraint b * c = a
                let quotient = self.new_temp_var();
                self.witnesses.push(quotient.clone());

                let numerator = self.convert_to_linear_combination(left)?;
                let denominator = self.convert_to_linear_combination(right)?;

                // constraint: denominator * quotient = numerator
                let constraint = R1CSConstraint {
                    a: denominator,
                    b: LinearCombination {
                        terms: vec![(quotient.clone(), BigInt::from(1))],
                    },
                    c: numerator,
                };
                warn!(
                    "PUSHING DIVISION CONSTRAINT #{}: {:?}",
                    self.constraints.len(),
                    constraint
                );
                self.constraints.push(constraint);

                Ok(LinearCombination {
                    terms: vec![(quotient, BigInt::from(1))],
                })
            }

            Operator::Assert => {
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;

                // resolve symbol map variables to their underlying linear combinations
                let resolved_left = self.resolve_symbol_map_variables(&left_lc);
                let resolved_right = self.resolve_symbol_map_variables(&right_lc);

                warn!(
                    "ASSERTION CONSTRAINT: {:?} * 1 = {:?}",
                    resolved_left, resolved_right
                );
                warn!("BEFORE RESOLUTION: {:?} * 1 = {:?}", left_lc, right_lc);

                let constraint = R1CSConstraint {
                    a: resolved_left,
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: resolved_right,
                };
                warn!(
                    "PUSHING ASSERTION CONSTRAINT #{}: {:?}",
                    self.constraints.len(),
                    constraint
                );
                self.constraints.push(constraint);

                Ok(LinearCombination { terms: vec![] })
            }

            Operator::Equal => {
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;

                let mut diff = left_lc;
                diff.add(&right_lc.negate());

                let inv = self.new_temp_var();
                let out = self.new_temp_var();
                self.witnesses.push(inv.clone());
                self.witnesses.push(out.clone());

                // constraint 1: out = -diff * inv + 1
                let mut out_expr = LinearCombination {
                    terms: vec![("ONE".to_string(), BigInt::from(1))],
                };
                let neg_product = self.new_temp_var();
                self.witnesses.push(neg_product.clone());

                self.constraints.push(R1CSConstraint {
                    a: diff.clone(),
                    b: LinearCombination {
                        terms: vec![(inv.clone(), BigInt::from(1))],
                    },
                    c: LinearCombination {
                        terms: vec![(neg_product.clone(), BigInt::from(1))],
                    },
                });

                out_expr.add(&LinearCombination {
                    terms: vec![(neg_product, BigInt::from(-1))],
                });

                self.constraints.push(R1CSConstraint {
                    a: LinearCombination {
                        terms: vec![(out.clone(), BigInt::from(1))],
                    },
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: out_expr,
                });

                // constraint 2: diff * out = 0
                self.constraints.push(R1CSConstraint {
                    a: diff,
                    b: LinearCombination {
                        terms: vec![(out.clone(), BigInt::from(1))],
                    },
                    c: LinearCombination { terms: vec![] },
                });

                self.enforce_boolean(&out);

                Ok(LinearCombination {
                    terms: vec![(out, BigInt::from(1))],
                })
            }

            Operator::NotEqual => {
                // NOT(a == b) = 1 - (a == b)
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;

                let mut diff = left_lc;
                diff.add(&right_lc.negate());

                let inv = self.new_temp_var();
                let eq_result = self.new_temp_var();
                self.witnesses.push(inv.clone());
                self.witnesses.push(eq_result.clone());

                // constraint 1: eq_result = -diff * inv + 1 (this is the IsZero check)
                let neg_product = self.new_temp_var();
                self.witnesses.push(neg_product.clone());

                self.constraints.push(R1CSConstraint {
                    a: diff.clone(),
                    b: LinearCombination {
                        terms: vec![(inv.clone(), BigInt::from(1))],
                    },
                    c: LinearCombination {
                        terms: vec![(neg_product.clone(), BigInt::from(1))],
                    },
                });

                let mut eq_expr = LinearCombination {
                    terms: vec![("ONE".to_string(), BigInt::from(1))],
                };
                eq_expr.add(&LinearCombination {
                    terms: vec![(neg_product, BigInt::from(-1))],
                });

                self.constraints.push(R1CSConstraint {
                    a: LinearCombination {
                        terms: vec![(eq_result.clone(), BigInt::from(1))],
                    },
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: eq_expr,
                });

                // constraint 2: diff * eq_result = 0
                self.constraints.push(R1CSConstraint {
                    a: diff,
                    b: LinearCombination {
                        terms: vec![(eq_result.clone(), BigInt::from(1))],
                    },
                    c: LinearCombination { terms: vec![] },
                });

                // constraint 3: neq_result = 1 - eq_result
                let neq_result = self.new_temp_var();
                self.witnesses.push(neq_result.clone());

                let mut neq_expr = LinearCombination {
                    terms: vec![("ONE".to_string(), BigInt::from(1))],
                };
                neq_expr.add(&LinearCombination {
                    terms: vec![(eq_result, BigInt::from(-1))],
                });

                self.constraints.push(R1CSConstraint {
                    a: LinearCombination {
                        terms: vec![(neq_result.clone(), BigInt::from(1))],
                    },
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: neq_expr,
                });

                self.enforce_boolean(&neq_result);

                Ok(LinearCombination {
                    terms: vec![(neq_result, BigInt::from(1))],
                })
            }

            Operator::Ge | Operator::Le | Operator::Gt | Operator::Lt => {
                self.convert_comparison(left, right, op)
            }

            Operator::And => {
                // result = a * b
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;

                let temp = self.new_temp_var();
                self.witnesses.push(temp.clone());

                // constraint: left * right = temp
                self.constraints.push(R1CSConstraint {
                    a: left_lc,
                    b: right_lc,
                    c: LinearCombination {
                        terms: vec![(temp.clone(), BigInt::from(1))],
                    },
                });

                self.enforce_boolean(&temp);

                Ok(LinearCombination {
                    terms: vec![(temp, BigInt::from(1))],
                })
            }

            Operator::Or => {
                // result = a + b - a*b
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;

                // first compute a * b
                let product_temp = self.new_temp_var();
                self.witnesses.push(product_temp.clone());

                self.constraints.push(R1CSConstraint {
                    a: left_lc.clone(),
                    b: right_lc.clone(),
                    c: LinearCombination {
                        terms: vec![(product_temp.clone(), BigInt::from(1))],
                    },
                });

                // then compute a + b - (a*b)
                let result_temp = self.new_temp_var();
                self.witnesses.push(result_temp.clone());

                let mut result_lc = left_lc;
                result_lc.add(&right_lc);
                result_lc.add(&LinearCombination {
                    terms: vec![(product_temp, BigInt::from(-1))],
                });

                // constraint: result = a + b - a*b (expressed as result * 1 = a + b - a*b)
                self.constraints.push(R1CSConstraint {
                    a: LinearCombination {
                        terms: vec![(result_temp.clone(), BigInt::from(1))],
                    },
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: result_lc,
                });

                self.enforce_boolean(&result_temp);

                Ok(LinearCombination {
                    terms: vec![(result_temp, BigInt::from(1))],
                })
            }

            Operator::Not => {
                // result = 1 - a
                let right_lc = self.convert_to_linear_combination(right)?;

                let temp = self.new_temp_var();
                self.witnesses.push(temp.clone());

                // constraint: temp = 1 - right (expressed as temp * 1 = 1 - right)
                let mut result_lc = LinearCombination {
                    terms: vec![("ONE".to_string(), BigInt::from(1))],
                };
                result_lc.add(&right_lc.negate());

                self.constraints.push(R1CSConstraint {
                    a: LinearCombination {
                        terms: vec![(temp.clone(), BigInt::from(1))],
                    },
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: result_lc,
                });

                self.enforce_boolean(&temp);

                Ok(LinearCombination {
                    terms: vec![(temp, BigInt::from(1))],
                })
            }
        }
    }

    fn convert_let_binding(
        &mut self,
        pattern: &Pattern,
        value: &Expression,
        body: &Expression,
    ) -> Result<LinearCombination, R1CSError> {
        debug!(
            "Converting let binding: {:?} = {:?} in {:?}",
            pattern, value, body
        );

        let saved_context = self.context.clone();

        if let Expression::ArrayLiteral(elements) = value {
            if let Pattern::Variable(array_name) = pattern {
                debug!("Binding array literal: {} = [...]", array_name);

                let mut element_vars = Vec::new();
                for (i, elem) in elements.iter().enumerate() {
                    let elem_var = format!("{}_{}", array_name, i);
                    let elem_lc = self.convert_to_linear_combination(elem)?;

                    self.witnesses.push(elem_var.clone());

                    self.constraints.push(R1CSConstraint {
                        a: LinearCombination {
                            terms: vec![(elem_var.clone(), BigInt::from(1))],
                        },
                        b: LinearCombination {
                            terms: vec![("ONE".to_string(), BigInt::from(1))],
                        },
                        c: elem_lc,
                    });

                    element_vars.push(elem_var);
                }

                self.arrays.insert(array_name.clone(), element_vars);

                let result = self.convert_to_linear_combination(body)?;

                self.context = saved_context;

                return Ok(result);
            }
        }

        let value_lc = self.convert_to_linear_combination(value)?;
        debug!("Value linear combination: {:?}", value_lc);

        match pattern {
            Pattern::Variable(name) => {
                debug!("Binding variable: {}", name);

                // check if this variable name is already in scope (shadowing)
                let is_shadowing = self.context.variables.contains_key(name);
                if is_shadowing {
                    debug!("Variable '{}' is shadowing existing variable", name);
                }

                // for let bindings, we need to handle two cases:
                // 1. Simple assignment: let x = expr (store substitution)
                // 2. Constraint generation for computed values (add to symbol map)

                if self.is_simple_variable_or_constant(&value_lc) {
                    debug!("Simple assignment: {} = {:?}", name, value_lc);

                    self.context.variables.insert(
                        name.clone(),
                        Type::Field {
                            constraint: crate::ast::ConstraintStatus::Constrained,
                            refinement: None,
                        },
                    );

                    // for simple assignments, instead of creating a constraint, store the substitution
                    // this allows the variable to be directly replaced with its value in other constraints
                    warn!("STORING SUBSTITUTION: {} -> {:?}", name, value_lc);
                    self.variable_substitutions.insert(name.clone(), value_lc);
                } else {
                    // complex expression - need to create a witness variable and constraint
                    debug!("Complex assignment: {} = {:?}", name, value_lc);

                    let var_index = self.get_next_variable_index();
                    warn!(
                        "INSERTING INTO SYMBOL MAP (complex): {} -> {}",
                        name, var_index
                    );
                    self.symbol_map.insert(name.clone(), var_index);

                    if !self.pub_inputs.contains(name) && !self.witnesses.contains(name) {
                        debug!("Adding {} as witness variable", name);
                        self.witnesses.push(name.clone());
                    }

                    self.context.variables.insert(
                        name.clone(),
                        Type::Field {
                            constraint: crate::ast::ConstraintStatus::Constrained,
                            refinement: None,
                        },
                    );

                    // create constraint: value_lc * 1 = name
                    // this defines name as the result of value_lc
                    debug!("Creating constraint: {:?} * 1 = {}", value_lc, name);

                    let resolved_value_lc = self.resolve_symbol_map_variables(&value_lc);

                    self.constraints.push(R1CSConstraint {
                        a: resolved_value_lc,
                        b: LinearCombination {
                            terms: vec![("ONE".to_string(), BigInt::from(1))],
                        },
                        c: LinearCombination {
                            terms: vec![(name.clone(), BigInt::from(1))],
                        },
                    });
                }
            }
            Pattern::Wildcard => {
                debug!("Wildcard pattern - evaluating value but not binding");
            }
            Pattern::Tuple(patterns) => {
                debug!("Tuple pattern with {} elements", patterns.len());

                // for tuple patterns, we need to decompose the value

                if let Expression::Variable(tuple_name) = value {
                    debug!("Destructuring tuple variable: {}", tuple_name);

                    for (i, sub_pattern) in patterns.iter().enumerate() {
                        match sub_pattern {
                            Pattern::Variable(var_name) => {
                                let component_name = format!("{}_{}", tuple_name, i);

                                debug!(
                                    "Binding {} to tuple component {}",
                                    var_name, component_name
                                );

                                self.variable_substitutions.insert(
                                    var_name.clone(),
                                    LinearCombination {
                                        terms: vec![(component_name.clone(), BigInt::from(1))],
                                    },
                                );

                                self.context.variables.insert(
                                    var_name.clone(),
                                    Type::Field {
                                        constraint: crate::ast::ConstraintStatus::Constrained,
                                        refinement: None,
                                    },
                                );
                            }
                            _ => {
                                warn!("Complex tuple sub-patterns not fully supported");
                            }
                        }
                    }
                } else {
                    warn!("Tuple destructuring of non-variable expressions not yet implemented");
                }
            }
            Pattern::Constructor(constructor_name, patterns) => {
                debug!(
                    "Constructor pattern: {} with {} sub-patterns",
                    constructor_name,
                    patterns.len()
                );
                warn!("Constructor patterns not supported in R1CS conversion");

                // constructor patterns would need special handling based on the type system
                // for now, just create a constraint that the value equals zero
                self.constraints.push(R1CSConstraint {
                    a: value_lc,
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(0))],
                    },
                });
            }
            Pattern::Literal(lit) => {
                debug!("Literal pattern: {}", lit);
                let lit_lc = LinearCombination {
                    terms: vec![("ONE".to_string(), BigInt::from(*lit))],
                };
                self.constraints.push(R1CSConstraint {
                    a: value_lc,
                    b: LinearCombination {
                        terms: vec![("ONE".to_string(), BigInt::from(1))],
                    },
                    c: lit_lc,
                });
            }
        }

        debug!("Processing let body: {:?}", body);
        let body_result = self.convert_to_linear_combination(body)?;

        debug!("Restoring context after let binding (preserving symbol map)");
        debug!("Symbol map before restore: {:?}", self.symbol_map);
        let current_symbol_map = self.symbol_map.clone();
        self.context = saved_context;
        self.symbol_map = current_symbol_map;
        debug!("Symbol map after restore: {:?}", self.symbol_map);

        Ok(body_result)
    }

    fn is_simple_variable_or_constant(&self, lc: &LinearCombination) -> bool {
        match lc.terms.len() {
            0 => true,
            1 => {
                let (var, coeff) = &lc.terms[0];
                (var == "ONE") || *coeff == BigInt::from(1)
            }
            _ => false,
        }
    }

    fn convert_match_expression(
        &mut self,
        value: &Expression,
        patterns: &[crate::ast::MatchPattern],
    ) -> Result<LinearCombination, R1CSError> {
        use crate::ast::Pattern;

        debug!(
            "Converting match expression with {} patterns",
            patterns.len()
        );

        let value_lc = self.convert_to_linear_combination(value)?;

        // for simple literal matching, we can implement this as a series of conditional checks
        // more complex patterns (constructors, tuples) require additional logic

        if patterns.is_empty() {
            return Err(R1CSError::InvalidExpression);
        }

        let all_simple = patterns.iter().all(|p| {
            matches!(
                p.pattern,
                Pattern::Literal(_) | Pattern::Wildcard | Pattern::Variable(_)
            )
        });

        if all_simple {
            let result_var = self.new_temp_var();
            self.witnesses.push(result_var.clone());

            let mut accumulated_result = LinearCombination { terms: vec![] };
            let mut remaining_probability = LinearCombination {
                terms: vec![("ONE".to_string(), BigInt::from(1))],
            };

            for (i, match_pattern) in patterns.iter().enumerate() {
                match &match_pattern.pattern {
                    Pattern::Literal(lit) => {
                        let lit_lc = LinearCombination {
                            terms: vec![("ONE".to_string(), BigInt::from(*lit))],
                        };

                        let mut diff = value_lc.clone();
                        diff.add(&lit_lc.negate());

                        let inv = self.new_temp_var();
                        let is_equal = self.new_temp_var();
                        self.witnesses.push(inv.clone());
                        self.witnesses.push(is_equal.clone());

                        let neg_product = self.new_temp_var();
                        self.witnesses.push(neg_product.clone());

                        self.constraints.push(R1CSConstraint {
                            a: diff.clone(),
                            b: LinearCombination {
                                terms: vec![(inv.clone(), BigInt::from(1))],
                            },
                            c: LinearCombination {
                                terms: vec![(neg_product.clone(), BigInt::from(1))],
                            },
                        });

                        let mut eq_expr = LinearCombination {
                            terms: vec![("ONE".to_string(), BigInt::from(1))],
                        };
                        eq_expr.add(&LinearCombination {
                            terms: vec![(neg_product, BigInt::from(-1))],
                        });

                        self.constraints.push(R1CSConstraint {
                            a: LinearCombination {
                                terms: vec![(is_equal.clone(), BigInt::from(1))],
                            },
                            b: LinearCombination {
                                terms: vec![("ONE".to_string(), BigInt::from(1))],
                            },
                            c: eq_expr,
                        });

                        self.constraints.push(R1CSConstraint {
                            a: diff,
                            b: LinearCombination {
                                terms: vec![(is_equal.clone(), BigInt::from(1))],
                            },
                            c: LinearCombination { terms: vec![] },
                        });

                        let branch_result =
                            self.convert_to_linear_combination(&match_pattern.body)?;

                        // result += is_equal * branch_result
                        let weighted_result = self.new_temp_var();
                        self.witnesses.push(weighted_result.clone());

                        self.constraints.push(R1CSConstraint {
                            a: LinearCombination {
                                terms: vec![(is_equal.clone(), BigInt::from(1))],
                            },
                            b: branch_result,
                            c: LinearCombination {
                                terms: vec![(weighted_result.clone(), BigInt::from(1))],
                            },
                        });

                        accumulated_result.add(&LinearCombination {
                            terms: vec![(weighted_result, BigInt::from(1))],
                        });

                        // remaining *= (1 - is_equal)
                        if i < patterns.len() - 1 {
                            let not_equal = self.new_temp_var();
                            self.witnesses.push(not_equal.clone());

                            let mut not_eq_expr = LinearCombination {
                                terms: vec![("ONE".to_string(), BigInt::from(1))],
                            };
                            not_eq_expr.add(&LinearCombination {
                                terms: vec![(is_equal, BigInt::from(-1))],
                            });

                            self.constraints.push(R1CSConstraint {
                                a: LinearCombination {
                                    terms: vec![(not_equal.clone(), BigInt::from(1))],
                                },
                                b: LinearCombination {
                                    terms: vec![("ONE".to_string(), BigInt::from(1))],
                                },
                                c: not_eq_expr,
                            });

                            let new_remaining = self.new_temp_var();
                            self.witnesses.push(new_remaining.clone());

                            self.constraints.push(R1CSConstraint {
                                a: remaining_probability.clone(),
                                b: LinearCombination {
                                    terms: vec![(not_equal, BigInt::from(1))],
                                },
                                c: LinearCombination {
                                    terms: vec![(new_remaining.clone(), BigInt::from(1))],
                                },
                            });

                            remaining_probability = LinearCombination {
                                terms: vec![(new_remaining, BigInt::from(1))],
                            };
                        }
                    }
                    Pattern::Wildcard | Pattern::Variable(_) => {
                        let saved_substitutions = self.variable_substitutions.clone();

                        if let Pattern::Variable(var_name) = &match_pattern.pattern {
                            debug!("Binding pattern variable '{}' to matched value", var_name);
                            self.variable_substitutions
                                .insert(var_name.clone(), value_lc.clone());
                        }

                        let branch_result =
                            self.convert_to_linear_combination(&match_pattern.body)?;

                        self.variable_substitutions = saved_substitutions;

                        let weighted_result = self.new_temp_var();
                        self.witnesses.push(weighted_result.clone());

                        self.constraints.push(R1CSConstraint {
                            a: remaining_probability.clone(),
                            b: branch_result,
                            c: LinearCombination {
                                terms: vec![(weighted_result.clone(), BigInt::from(1))],
                            },
                        });

                        accumulated_result.add(&LinearCombination {
                            terms: vec![(weighted_result, BigInt::from(1))],
                        });

                        break;
                    }
                    _ => {}
                }
            }

            // constrain result variable to equal accumulated result
            self.constraints.push(R1CSConstraint {
                a: LinearCombination {
                    terms: vec![(result_var.clone(), BigInt::from(1))],
                },
                b: LinearCombination {
                    terms: vec![("ONE".to_string(), BigInt::from(1))],
                },
                c: accumulated_result,
            });

            Ok(LinearCombination {
                terms: vec![(result_var, BigInt::from(1))],
            })
        } else {
            // complex patterns (tuples, constructors) not yet supported
            warn!("Complex match patterns (tuples, constructors) not yet fully supported in R1CS");
            Ok(LinearCombination { terms: vec![] })
        }
    }

    fn convert_function_call(
        &mut self,
        function: &str,
        arguments: &[Expression],
    ) -> Result<LinearCombination, R1CSError> {
        if function == "decompose" {
            return self.convert_decompose(arguments);
        }

        if let Some((params, body)) = self.function_defs.get(function).cloned() {
            debug!(
                "Inlining function '{}' with {} arguments",
                function,
                arguments.len()
            );

            if arguments.len() != params.len() {
                return Err(R1CSError::InvalidArgument(format!(
                    "Function '{}' expects {} arguments, got {}",
                    function,
                    params.len(),
                    arguments.len()
                )));
            }

            let saved_substitutions = self.variable_substitutions.clone();

            for (param, arg) in params.iter().zip(arguments.iter()) {
                let arg_lc = self.convert_to_linear_combination(arg)?;
                self.variable_substitutions
                    .insert(param.name.clone(), arg_lc);
            }

            let result = self.convert_to_linear_combination(&body)?;

            self.variable_substitutions = saved_substitutions;

            Ok(result)
        } else {
            warn!("Function '{}' not found in function definitions", function);
            Ok(LinearCombination { terms: vec![] })
        }
    }

    fn convert_decompose(
        &mut self,
        arguments: &[Expression],
    ) -> Result<LinearCombination, R1CSError> {
        if arguments.is_empty() || arguments.len() > 2 {
            return Err(R1CSError::InvalidArgument(
                "decompose expects 1 or 2 arguments".to_string(),
            ));
        }

        let input_var = match &arguments[0] {
            Expression::Variable(name) => name,
            _ => {
                return Err(R1CSError::InvalidArgument(
                    "decompose expects a variable".to_string(),
                ))
            }
        };

        let bit_width = if arguments.len() == 2 {
            match &arguments[1] {
                Expression::Number(n) => *n as usize,
                _ => {
                    return Err(R1CSError::InvalidArgument(
                        "decompose bit width must be a number".to_string(),
                    ))
                }
            }
        } else {
            8
        };

        let mut sum_terms = Vec::new();

        for i in 0..bit_width {
            let bit = format!("{}_bit_{}", input_var, i);

            if !self.witnesses.contains(&bit) {
                self.witnesses.push(bit.clone());
            }

            // constraint: bit * (1 - bit) = 0 (ensures bit is 0 or 1)
            self.constraints.push(R1CSConstraint {
                a: LinearCombination {
                    terms: vec![(bit.clone(), BigInt::from(1))],
                },
                b: LinearCombination {
                    terms: vec![
                        ("ONE".to_string(), BigInt::from(1)),
                        (bit.clone(), BigInt::from(-1)),
                    ],
                },
                c: LinearCombination { terms: vec![] },
            });

            sum_terms.push((bit, BigInt::from(1) << i));
        }

        // constraint: sum of weighted bits = original value
        self.constraints.push(R1CSConstraint {
            a: LinearCombination {
                terms: sum_terms.clone(),
            },
            b: LinearCombination {
                terms: vec![("ONE".to_string(), BigInt::from(1))],
            },
            c: LinearCombination {
                terms: vec![(input_var.clone(), BigInt::from(1))],
            },
        });

        Ok(LinearCombination { terms: sum_terms })
    }

    fn convert_comparison(
        &mut self,
        left: &Expression,
        right: &Expression,
        op: &Operator,
    ) -> Result<LinearCombination, R1CSError> {
        debug!("Converting comparison: {:?} {:?} {:?}", left, op, right);

        let diff_var = self.new_temp_var();
        let result_var = self.new_temp_var();

        self.witnesses.push(diff_var.clone());
        self.witnesses.push(result_var.clone());

        let (left_lc, right_lc) = match op {
            Operator::Gt => {
                // for a > b, swap to compute b < a
                (
                    self.convert_to_linear_combination(right)?,
                    self.convert_to_linear_combination(left)?,
                )
            }
            Operator::Ge => {
                // for a >= b, compute b < a+1, i.e., b < (a+1)
                // so diff = 2^252 + b - (a+1) = 2^252 + b - a - 1
                let mut right_tmp = self.convert_to_linear_combination(left)?;
                // add 1 to right (which represents 'a'), so right becomes a+1
                right_tmp.terms.push(("ONE".to_string(), BigInt::from(1)));
                (self.convert_to_linear_combination(right)?, right_tmp)
            }
            Operator::Lt => {
                // for a < b, compute as-is
                (
                    self.convert_to_linear_combination(left)?,
                    self.convert_to_linear_combination(right)?,
                )
            }
            Operator::Le => {
                // for a <= b, compute a < b+1
                // so diff = 2^252 + a - (b+1) = 2^252 + a - b - 1
                let mut right_tmp = self.convert_to_linear_combination(right)?;
                // add 1 to right (which represents 'b'), so right becomes b+1
                right_tmp.terms.push(("ONE".to_string(), BigInt::from(1)));
                (self.convert_to_linear_combination(left)?, right_tmp)
            }
            _ => unreachable!(),
        };

        // cnstraint: diff = 2^252 + left - right
        // the 2^252 offset is crucial for proper strict < handling (like Circom's LessThan)
        // this shifts the range so that equality gives bit 252 = 1 (not 0)
        const OFFSET_BITS: i64 = 252;
        let offset = BigInt::from(1) << OFFSET_BITS; // 2^252

        let mut diff_lc = left_lc;
        diff_lc.add(&right_lc.negate());
        diff_lc.terms.push(("ONE".to_string(), offset));

        self.constraints.push(R1CSConstraint {
            a: LinearCombination {
                terms: vec![(diff_var.clone(), BigInt::from(1))],
            },
            b: LinearCombination {
                terms: vec![("ONE".to_string(), BigInt::from(1))],
            },
            c: diff_lc,
        });

        const COMPARISON_BITS: i64 = 253;
        self.convert_decompose(&[
            Expression::Variable(diff_var.clone()),
            Expression::Number(COMPARISON_BITS),
        ])?;

        let sign_bit = format!("{}_bit_{}", diff_var, COMPARISON_BITS - 1);

        self.constraints.push(R1CSConstraint {
            a: LinearCombination {
                terms: vec![(result_var.clone(), BigInt::from(1))],
            },
            b: LinearCombination {
                terms: vec![("ONE".to_string(), BigInt::from(1))],
            },
            c: LinearCombination {
                terms: vec![
                    ("ONE".to_string(), BigInt::from(1)),
                    (sign_bit, BigInt::from(-1)),
                ],
            },
        });
        debug!("Comparison result variable: {}", result_var);
        self.enforce_boolean(&result_var);
        Ok(LinearCombination {
            terms: vec![(result_var, BigInt::from(1))],
        })
    }

    #[allow(clippy::type_complexity)]
    pub fn get_matrices(&self) -> (Vec<Vec<BigInt>>, Vec<Vec<BigInt>>, Vec<Vec<BigInt>>) {
        let n_vars = self.pub_inputs.len() + self.witnesses.len() + self.temp_var_counter + 1;
        let n_constraints = self.constraints.len();

        let mut a_matrix = vec![vec![BigInt::from(0); n_vars]; n_constraints];
        let mut b_matrix = vec![vec![BigInt::from(0); n_vars]; n_constraints];
        let mut c_matrix = vec![vec![BigInt::from(0); n_vars]; n_constraints];

        for (i, constraint) in self.constraints.iter().enumerate() {
            for (var, coeff) in &constraint.a.terms {
                let var_idx = self.get_variable_index(var);
                if var_idx < n_vars {
                    a_matrix[i][var_idx] = coeff.clone();
                }
            }

            for (var, coeff) in &constraint.b.terms {
                let var_idx = self.get_variable_index(var);
                if var_idx < n_vars {
                    b_matrix[i][var_idx] = coeff.clone();
                }
            }

            for (var, coeff) in &constraint.c.terms {
                let var_idx = self.get_variable_index(var);
                if var_idx < n_vars {
                    c_matrix[i][var_idx] = coeff.clone();
                }
            }
        }

        (a_matrix, b_matrix, c_matrix)
    }

    fn get_variable_index(&self, var: &str) -> usize {
        if var == "ONE" {
            return 0;
        }

        if let Some(pos) = self.pub_inputs.iter().position(|x| x == var) {
            return pos + 1;
        }

        if let Some(pos) = self.witnesses.iter().position(|x| x == var) {
            return self.pub_inputs.len() + pos + 1;
        }

        if let Some(index) = self.symbol_map.get(var) {
            return *index;
        }

        warn!(
            "Unknown variable: {} (pub_inputs: {:?}, witnesses: {:?}, symbol_map: {:?})",
            var, self.pub_inputs, self.witnesses, self.symbol_map
        );
        0
    }

    fn get_next_variable_index(&self) -> usize {
        self.pub_inputs.len() + self.witnesses.len() + self.temp_var_counter + 1
    }

    fn resolve_symbol_map_variables(&self, lc: &LinearCombination) -> LinearCombination {
        let mut resolved_terms = Vec::new();

        for (var, coeff) in &lc.terms {
            if let Some(substitution) = self.variable_substitutions.get(var) {
                for (sub_var, sub_coeff) in &substitution.terms {
                    resolved_terms.push((sub_var.clone(), coeff * sub_coeff));
                }
            } else {
                resolved_terms.push((var.clone(), coeff.clone()));
            }
        }

        LinearCombination {
            terms: resolved_terms,
        }
    }

    pub fn get_constraints(&self) -> &Vec<R1CSConstraint> {
        &self.constraints
    }
}

impl LinearCombination {
    fn add(&mut self, other: &LinearCombination) {
        self.terms.extend(other.terms.clone());
    }

    fn negate(&self) -> LinearCombination {
        LinearCombination {
            terms: self
                .terms
                .iter()
                .map(|(var, coeff)| (var.clone(), -coeff.clone()))
                .collect(),
        }
    }
}

impl std::fmt::Display for R1CSConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        for (i, (var, coeff)) in self.a.terms.iter().enumerate() {
            if i > 0 {
                write!(f, " + ")?;
            }
            write!(f, "{}*{}", coeff, var)?;
        }
        write!(f, ") * (")?;
        for (i, (var, coeff)) in self.b.terms.iter().enumerate() {
            if i > 0 {
                write!(f, " + ")?;
            }
            write!(f, "{}*{}", coeff, var)?;
        }
        write!(f, ") = (")?;
        for (i, (var, coeff)) in self.c.terms.iter().enumerate() {
            if i > 0 {
                write!(f, " + ")?;
            }
            write!(f, "{}*{}", coeff, var)?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for R1CSError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            R1CSError::NonQuadratic => write!(f, "Non-quadratic constraint"),
            R1CSError::InvalidExpression => write!(f, "Invalid expression in R1CS"),
            R1CSError::FileError => write!(f, "Error writing R1CS file"),
            R1CSError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
            R1CSError::InvalidFunction(func) => write!(f, "Invalid function: {}", func),
            R1CSError::InvalidArgument(arg) => write!(f, "Invalid argument: {}", arg),
            R1CSError::TypeError(msg) => write!(f, "Type error: {}", msg),
            R1CSError::UnknownVariable(var) => write!(f, "Unknown variable: {}", var),
        }
    }
}

pub fn read_r1cs_file(path: &PathBuf) -> std::io::Result<R1CSGenerator> {
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != b"lof-r1cs" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid magic bytes - not a lof-r1cs file",
        ));
    }

    let mut version = [0u8; 4];
    reader.read_exact(&mut version)?;
    if u32::from_le_bytes(version) != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unsupported r1cs version",
        ));
    }

    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    let pub_inputs_count = u32::from_le_bytes(buf);

    reader.read_exact(&mut buf)?;
    let witnesses_count = u32::from_le_bytes(buf);

    reader.read_exact(&mut buf)?;
    let constraints_count = u32::from_le_bytes(buf);

    let mut pub_inputs = Vec::new();
    for _ in 0..pub_inputs_count {
        reader.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf) as usize;
        let mut name = vec![0u8; len];
        reader.read_exact(&mut name)?;
        pub_inputs.push(
            String::from_utf8(name)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        );
    }

    let mut witnesses = Vec::new();
    for _ in 0..witnesses_count {
        reader.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf) as usize;
        let mut name = vec![0u8; len];
        reader.read_exact(&mut name)?;
        witnesses.push(
            String::from_utf8(name)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        );
    }

    let mut constraints = Vec::new();
    for _ in 0..constraints_count {
        let a = read_linear_combination(&mut reader)?;
        let b = read_linear_combination(&mut reader)?;
        let c = read_linear_combination(&mut reader)?;
        constraints.push(R1CSConstraint { a, b, c });
    }

    Ok(R1CSGenerator {
        constraints,
        temp_var_counter: 0,
        symbol_map: HashMap::new(),
        variable_substitutions: HashMap::new(),
        pub_inputs,
        witnesses,
        context: R1CSContext {
            variables: HashMap::new(),
        },
        function_defs: HashMap::new(),
        arrays: HashMap::new(),
        boolean_vars: HashSet::new(),
    })
}

fn read_linear_combination<R: Read>(reader: &mut R) -> std::io::Result<LinearCombination> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    let terms_count = u32::from_le_bytes(buf);

    let mut terms = Vec::new();
    for _ in 0..terms_count {
        reader.read_exact(&mut buf)?;
        let var_idx = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let bytes_len = u32::from_le_bytes(buf) as usize;
        let mut bytes = vec![0u8; bytes_len];
        reader.read_exact(&mut bytes)?;

        let coeff = BigInt::from_signed_bytes_le(&bytes);

        terms.push((format!("var_{}", var_idx), coeff));
    }

    Ok(LinearCombination { terms })
}
