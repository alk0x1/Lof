use std::{collections::HashMap, io::{Read, Seek, Write}, path::PathBuf};
use crate::ast::{Expression, Operator, Pattern, Visibility, Type};
use tracing::{info, warn, debug};
use std::fmt;

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
    LinearityViolation(String),
}

#[derive(Debug, Clone)]
pub struct R1CSConstraint {
    pub a: LinearCombination,
    pub b: LinearCombination,
    pub c: LinearCombination,
}

#[derive(Debug, Clone)]
pub struct LinearCombination {
    pub terms: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct R1CSContext {
    /// Current scope variables
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
        }
    }
    
    pub fn write_r1cs_file(&self, source_path: &PathBuf) -> std::io::Result<u64> {
        let mut r1cs_path = source_path.parent()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::Other, 
                "Could not determine parent directory"
            ))?.to_path_buf();
        
        let file_stem = source_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        r1cs_path.push(format!("{}.r1cs", file_stem));
        
        info!("Writing R1CS file to: {}", r1cs_path.display());

        let file = std::fs::File::create(&r1cs_path)?;
        let mut writer = std::io::BufWriter::new(file);

        // Magic header
        writer.write_all(b"lof-r1cs")?;
        writer.write_all(&1u32.to_le_bytes())?; // Version

        // Counts
        writer.write_all(&(self.pub_inputs.len() as u32).to_le_bytes())?;
        writer.write_all(&(self.witnesses.len() as u32).to_le_bytes())?;
        writer.write_all(&(self.constraints.len() as u32).to_le_bytes())?;

        // Public inputs
        for input in &self.pub_inputs {
            writer.write_all(&(input.len() as u32).to_le_bytes())?;
            writer.write_all(input.as_bytes())?;
        }

        // Witnesses
        for witness in &self.witnesses {
            writer.write_all(&(witness.len() as u32).to_le_bytes())?;
            writer.write_all(witness.as_bytes())?;
        }

        // Constraints
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

    fn write_linear_combination<W: Write + Seek>(&self, writer: &mut W, lc: &LinearCombination) -> std::io::Result<()> {
        writer.write_all(&(lc.terms.len() as u32).to_le_bytes())?;
        
        for (var, coeff) in &lc.terms {
            let idx = self.get_variable_index(var);
            writer.write_all(&(idx as u32).to_le_bytes())?;
            writer.write_all(&(coeff).to_le_bytes())?;
        }
        
        Ok(())
    }

    fn new_temp_var(&mut self) -> String {
        let var = format!("t_{}", self.temp_var_counter);
        self.temp_var_counter += 1;
        var
    }

    pub fn convert_proof(&mut self, expr: &Expression) -> Result<(), R1CSError> {
        match expr {
            Expression::Proof { name, signals, body, .. } => {
                debug!("Converting proof '{}' to R1CS", name);
                
                // Process signals and add them to appropriate collections
                for signal in signals {
                    match signal.visibility {
                        Visibility::Input | Visibility::Output => {
                            self.pub_inputs.push(signal.name.clone());
                            self.context.variables.insert(signal.name.clone(), signal.typ.clone());
                        }
                        Visibility::Witness => {
                            self.witnesses.push(signal.name.clone());
                            self.context.variables.insert(signal.name.clone(), signal.typ.clone());
                        }
                    }
                }

                // Convert the proof body to constraints
                let _result = self.convert_to_linear_combination(body)?;
                
                debug!("Generated {} constraints for proof '{}'", self.constraints.len(), name);
                Ok(())
            }
            _ => Err(R1CSError::InvalidArgument("Expected proof expression".to_string()))
        }
    }

    fn convert_to_linear_combination(&mut self, expr: &Expression) -> Result<LinearCombination, R1CSError> {
        match expr {
            Expression::Variable(name) => {
                // Check if this variable is a symbol map alias for another linear combination
                // If so, we should resolve to the underlying computation for constraint solving
                Ok(LinearCombination {
                    terms: vec![(name.clone(), 1)]
                })
            }
            
            Expression::Number(n) => {
                Ok(LinearCombination {
                    terms: vec![("ONE".to_string(), *n)]
                })
            }
            
            Expression::BinaryOp { left, op, right } => {
                self.convert_binary_op(left, op, right)
            }
            
            Expression::Assert(condition) => {
                // For assert statements, just process the condition
                // The condition itself (if it's an assertion operator) will create the constraint
                let cond_lc = self.convert_to_linear_combination(condition)?;
                
                // Don't create an additional constraint here - the assertion operator handles it
                warn!("PROCESSED ASSERT EXPRESSION, condition result: {:?}", cond_lc);
                
                Ok(LinearCombination { terms: vec![] })
            }
            
            Expression::Let { pattern, value, body } => {
                self.convert_let_binding(pattern, value, body)
            }
            
            Expression::Match { value, patterns } => {
                self.convert_match_expression(value, patterns)
            }
            
            Expression::Block { statements, final_expr } => {
                // Create new scope but preserve consumed variables from outer scope
                let saved_variables = self.context.variables.clone();
                
                // Process statements
                for stmt in statements {
                    self.convert_to_linear_combination(stmt)?;
                }
                
                // Process final expression if any
                let result = if let Some(expr) = final_expr {
                    self.convert_to_linear_combination(expr)?
                } else {
                    LinearCombination { terms: vec![] }
                };
                
                self.context.variables = saved_variables;
                
                Ok(result)
            }
            
            Expression::Tuple(elements) => {
                // For R1CS, we might need to flatten tuples or handle them specially
                // For now, just process each element
                for elem in elements {
                    self.convert_to_linear_combination(elem)?;
                }
                Ok(LinearCombination { terms: vec![] })
            }
            
            Expression::FunctionCall { function, arguments } => {
                self.convert_function_call(function, arguments)
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
        right: &Expression
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
                // Multiplication requires a new constraint: a * b = c
                let temp = self.new_temp_var();
                let a = self.convert_to_linear_combination(left)?;
                let b = self.convert_to_linear_combination(right)?;
                
                let constraint = R1CSConstraint {
                    a,
                    b,
                    c: LinearCombination {
                        terms: vec![(temp.clone(), 1)]
                    }
                };
                warn!("PUSHING MULTIPLICATION CONSTRAINT #{}: {:?}", self.constraints.len(), constraint);
                self.constraints.push(constraint);
                
                Ok(LinearCombination {
                    terms: vec![(temp, 1)]
                })
            }
            
            Operator::Assert => {
                // Assert operator: left === right
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;
                
                // Resolve symbol map variables to their underlying linear combinations
                let resolved_left = self.resolve_symbol_map_variables(&left_lc);
                let resolved_right = self.resolve_symbol_map_variables(&right_lc);
                
                warn!("ASSERTION CONSTRAINT: {:?} * 1 = {:?}", resolved_left, resolved_right);
                warn!("BEFORE RESOLUTION: {:?} * 1 = {:?}", left_lc, right_lc);

                let constraint = R1CSConstraint {
                    a: resolved_left,
                    b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                    c: resolved_right
                };
                warn!("PUSHING ASSERTION CONSTRAINT #{}: {:?}", self.constraints.len(), constraint);
                self.constraints.push(constraint);
                
                Ok(LinearCombination { terms: vec![] })
            }
            
            Operator::Equal | Operator::NotEqual => {
                // Comparison operations - need special handling for R1CS
                // For now, create a boolean result
                let temp = self.new_temp_var();
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;
                
                // Create constraint for equality check
                // Note: This implements basic equality verification via difference constraint
                let mut diff = left_lc;
                diff.add(&right_lc.negate());
                
                self.constraints.push(R1CSConstraint {
                    a: diff,
                    b: LinearCombination { terms: vec![(temp.clone(), 1)] },
                    c: LinearCombination { terms: vec![] }
                });
                
                Ok(LinearCombination {
                    terms: vec![(temp, 1)]
                })
            }
            
            Operator::Ge | Operator::Le | Operator::Gt | Operator::Lt => {
                self.convert_comparison(left, right, op)
            }
            
            _ => Err(R1CSError::UnsupportedOperation(format!("Operator {:?} not supported in R1CS", op)))
        }
    }

    fn convert_let_binding(
      &mut self,
      pattern: &Pattern,
      value: &Expression,
      body: &Expression
  ) -> Result<LinearCombination, R1CSError> {
      debug!("Converting let binding: {:?} = {:?} in {:?}", pattern, value, body);
      
      // Save current context for proper scoping
      let saved_context = self.context.clone();
      
      let value_lc = self.convert_to_linear_combination(value)?;
      debug!("Value linear combination: {:?}", value_lc);
      
      // Bind pattern variables in current scope
      match pattern {
          Pattern::Variable(name) => {
              debug!("Binding variable: {}", name);

              // Check if this variable name is already in scope (shadowing)
              let is_shadowing = self.context.variables.contains_key(name);
              if is_shadowing {
                  debug!("Variable '{}' is shadowing existing variable", name);
              }
              
              // For let bindings,we need to handle two cases:
              // 1. Simple assignment: let x = expr (store substitution)
              // 2. Constraint generation for computed values (add to symbol map)
              
              if self.is_simple_variable_or_constant(&value_lc) {
                  debug!("Simple assignment: {} = {:?}", name, value_lc);
                  
                  // Add to context
                  self.context.variables.insert(name.clone(), Type::Field);
                  
                  // For simple assignments, instead of creating a constraint, store the substitution
                  // This allows the variable to be directly replaced with its value in other constraints
                  warn!("STORING SUBSTITUTION: {} -> {:?}", name, value_lc);
                  self.variable_substitutions.insert(name.clone(), value_lc);
              } else {
                  // Complex expression - need to create a witness variable and constraint
                  debug!("Complex assignment: {} = {:?}", name, value_lc);
                  
                  // Get variable index and add to symbol map for complex assignments
                  let var_index = self.get_next_variable_index();
                  warn!("INSERTING INTO SYMBOL MAP (complex): {} -> {}", name, var_index);
                  self.symbol_map.insert(name.clone(), var_index);
                  
                  // Add as witness if not already a public input
                  if !self.pub_inputs.contains(name) && !self.witnesses.contains(name) {
                      debug!("Adding {} as witness variable", name);
                      self.witnesses.push(name.clone());
                  }
                  
                  self.context.variables.insert(name.clone(), Type::Field);
                  
                  // Create constraint: name * 1 = value_lc
                  debug!("Creating constraint: {} * 1 = {:?}", name, value_lc);
                  
                  let a = self.convert_to_linear_combination(value)?;

                  self.constraints.push(R1CSConstraint {
                      a,
                      b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                      c: value_lc
                  });
              }
          }
          Pattern::Wildcard => {
              debug!("Wildcard pattern - evaluating value but not binding");
              // Just evaluate the value for side effects, don't bind anything
          }
          Pattern::Tuple(patterns) => {
              debug!("Tuple pattern with {} elements", patterns.len());
              
              // For tuple patterns, we need to decompose the value
              // Note: Current implementation creates separate witnesses for each tuple element
              // Future enhancement: implement proper tuple value decomposition
              
              // For now, we'll create constraints for each pattern element
              // assuming the value is a tuple of the same size
              for (i, sub_pattern) in patterns.iter().enumerate() {
                  match sub_pattern {
                      Pattern::Variable(var_name) => {
                          // Create a witness for each tuple element
                          let element_name = format!("{}_{}", var_name, i);
                          
                          if !self.pub_inputs.contains(&element_name) && !self.witnesses.contains(&element_name) {
                              self.witnesses.push(element_name.clone());
                          }
                          
                          self.context.variables.insert(element_name.clone(), Type::Field);
                          
                          // For simplicity, create a constraint that this element equals zero
                          // A full implementation would need proper tuple handling
                          self.constraints.push(R1CSConstraint {
                              a: LinearCombination { terms: vec![(element_name, 1)] },
                              b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                              c: LinearCombination { terms: vec![("ONE".to_string(), 0)] }
                          });
                      }
                      _ => {
                          warn!("Complex tuple sub-patterns not fully supported");
                      }
                  }
              }
              
              warn!("Tuple patterns not fully implemented in R1CS conversion");
          }
          Pattern::Constructor(constructor_name, patterns) => {
              debug!("Constructor pattern: {} with {} sub-patterns", constructor_name, patterns.len());
              warn!("Constructor patterns not supported in R1CS conversion");
              
              // Constructor patterns would need special handling based on the type system
              // For now, just create a constraint that the value equals zero
              self.constraints.push(R1CSConstraint {
                  a: value_lc,
                  b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                  c: LinearCombination { terms: vec![("ONE".to_string(), 0)] }
              });
          }
          Pattern::Literal(lit) => {
              debug!("Literal pattern: {}", lit);
              // Create constraint that value equals the literal
              let lit_lc = LinearCombination { terms: vec![("ONE".to_string(), *lit)] };
              self.constraints.push(R1CSConstraint {
                  a: value_lc,
                  b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                  c: lit_lc
              });
          }
      }
      
      // Process body in the updated context
      debug!("Processing let body: {:?}", body);
      let body_result = self.convert_to_linear_combination(body)?;
      
      // Restore the previous context (lexical scoping) but preserve symbol map
      // This ensures that variables bound in the let don't leak to outer scope
      // but their symbol mappings persist for R1CS constraint generation
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
          0 => true, // Empty (zero)
          1 => {
              let (var, coeff) = &lc.terms[0];
              // Simple if it's just ONE (constant) or a single variable with coefficient 1
              (var == "ONE") || (*coeff == 1 && var != "ONE")
          }
          _ => false // Multiple terms = complex expression
      }
  }

    fn convert_match_expression(
        &mut self,
        _value: &Expression,
        _patterns: &[crate::ast::MatchPattern]
    ) -> Result<LinearCombination, R1CSError> {
        // Match expressions are complex for R1CS - would need special handling
        warn!("Match expressions not yet supported in R1CS conversion");
        Ok(LinearCombination { terms: vec![] })
    }

    fn convert_function_call(
        &mut self,
        function: &str,
        arguments: &[Expression]
    ) -> Result<LinearCombination, R1CSError> {
        match function {
            "decompose" => self.convert_decompose(arguments),
            _ => {
                warn!("Function '{}' not supported in R1CS conversion", function);
                Ok(LinearCombination { terms: vec![] })
            }
        }
    }

    fn convert_decompose(&mut self, arguments: &[Expression]) -> Result<LinearCombination, R1CSError> {
        if arguments.is_empty() || arguments.len() > 2 {
            return Err(R1CSError::InvalidArgument("decompose expects 1 or 2 arguments".to_string()));
        }

        let input_var = match &arguments[0] {
            Expression::Variable(name) => name,
            _ => return Err(R1CSError::InvalidArgument("decompose expects a variable".to_string()))
        };

        // Determine bit width - default to 8 for backward compatibility
        let bit_width = if arguments.len() == 2 {
            match &arguments[1] {
                Expression::Number(n) => *n as usize,
                _ => return Err(R1CSError::InvalidArgument("decompose bit width must be a number".to_string()))
            }
        } else {
            8
        };

        let mut sum_terms = Vec::new();
        
        // Create bit variables and constraints for the specified bit width
        for i in 0..bit_width {
            let bit = format!("{}_bit_{}", input_var, i);
            
            // Only add to witnesses if not already present
            if !self.witnesses.contains(&bit) {
                self.witnesses.push(bit.clone());
            }
            
            // Constraint: bit * (1 - bit) = 0 (ensures bit is 0 or 1)
            self.constraints.push(R1CSConstraint {
                a: LinearCombination { terms: vec![(bit.clone(), 1)] },
                b: LinearCombination { 
                    terms: vec![("ONE".to_string(), 1), (bit.clone(), -1)] 
                },
                c: LinearCombination { terms: vec![] }
            });

            sum_terms.push((bit, 1_i64 << i));
        }

        // Constraint: sum of weighted bits = original value
        self.constraints.push(R1CSConstraint {
            a: LinearCombination { terms: sum_terms.clone() },
            b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
            c: LinearCombination { terms: vec![(input_var.clone(), 1)] }
        });

        Ok(LinearCombination { terms: sum_terms })
    }

    fn convert_comparison(
        &mut self, 
        left: &Expression, 
        right: &Expression,
        op: &Operator
    ) -> Result<LinearCombination, R1CSError> {
        debug!("Converting comparison: {:?} {:?} {:?}", left, op, right);
        
        // Step 1: Create witnesses for the difference and result
        let diff_var = self.new_temp_var();
        let result_var = self.new_temp_var(); // Boolean result (0 or 1)
        
        // Add these as witnesses
        self.witnesses.push(diff_var.clone());
        self.witnesses.push(result_var.clone());
        
        // Step 2: Compute the difference based on comparison type
        let (left_lc, right_lc) = match op {
            Operator::Ge | Operator::Gt => {
                // For a >= b or a > b, check if a - b >= 0
                (self.convert_to_linear_combination(left)?, 
                 self.convert_to_linear_combination(right)?)
            }
            Operator::Le | Operator::Lt => {
                // For a <= b or a < b, check if b - a >= 0  
                (self.convert_to_linear_combination(right)?, 
                 self.convert_to_linear_combination(left)?)
            }
            _ => unreachable!()
        };
        
        // Step 3: Constraint: diff = left - right
        let mut diff_lc = left_lc;
        diff_lc.add(&right_lc.negate());
        
        self.constraints.push(R1CSConstraint {
            a: LinearCombination { terms: vec![(diff_var.clone(), 1)] },
            b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
            c: diff_lc
        });
        
        // Step 4: Decompose the difference into bits (64 bits for now)
        // WARNING: This implementation only supports comparisons for values < 2^63
        // For full field range, use 252-bit decomposition with big integer coefficients
        // This covers a very large range while avoiding overflow issues
        self.convert_decompose(&[
            Expression::Variable(diff_var.clone()), 
            Expression::Number(64)
        ])?;
        
        // Step 5: Check the sign bit (bit 63) to determine if positive
        let sign_bit = format!("{}_bit_63", diff_var);
        
        // Step 6: result = 1 - sign_bit (if sign_bit = 0 then positive, result = 1)
        self.constraints.push(R1CSConstraint {
            a: LinearCombination { terms: vec![(result_var.clone(), 1)] },
            b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
            c: LinearCombination { 
                terms: vec![("ONE".to_string(), 1), (sign_bit, -1)] 
            }
        });
        
        // Handle strict vs non-strict comparisons
        match op {
            Operator::Gt | Operator::Lt => {
                // For strict comparison, also need diff != 0
                // This would require additional zero-check logic
                // For now, treat same as >= and <=
                warn!("Strict comparisons (> and <) not fully implemented, treating as >= and <=");
            }
            _ => {}
        }
        
        debug!("Comparison result variable: {}", result_var);
        Ok(LinearCombination { terms: vec![(result_var, 1)] })
    }

    pub fn get_matrices(&self) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
        let n_vars = self.pub_inputs.len() + self.witnesses.len() + self.temp_var_counter + 1; // +1 for ONE
        let n_constraints = self.constraints.len();

        let mut a_matrix = vec![vec![0; n_vars]; n_constraints];
        let mut b_matrix = vec![vec![0; n_vars]; n_constraints];
        let mut c_matrix = vec![vec![0; n_vars]; n_constraints];

        for (i, constraint) in self.constraints.iter().enumerate() {
            for (var, coeff) in &constraint.a.terms {
                let var_idx = self.get_variable_index(var);
                if var_idx < n_vars {
                    a_matrix[i][var_idx] = *coeff;
                }
            }

            for (var, coeff) in &constraint.b.terms {
                let var_idx = self.get_variable_index(var);
                if var_idx < n_vars {
                    b_matrix[i][var_idx] = *coeff;
                }
            }

            for (var, coeff) in &constraint.c.terms {
                let var_idx = self.get_variable_index(var);
                if var_idx < n_vars {
                    c_matrix[i][var_idx] = *coeff;
                }
            }
        }

        (a_matrix, b_matrix, c_matrix)
    }

    fn get_variable_index(&self, var: &str) -> usize {
        if var == "ONE" {
            return 0;
        }

        // Check public inputs first
        if let Some(pos) = self.pub_inputs.iter().position(|x| x == var) {
            return pos + 1;  // +1 because ONE is at 0
        }

        // Check witnesses 
        if let Some(pos) = self.witnesses.iter().position(|x| x == var) {
            return self.pub_inputs.len() + pos + 1;  // +1 for ONE
        }

        // Check symbol_map for let-bound variables
        if let Some(index) = self.symbol_map.get(var) {
            return *index;
        }

        // Handle temporary variables
        if var.starts_with("t_") {
            if let Ok(num) = var[2..].parse::<usize>() {
                return self.pub_inputs.len() + self.witnesses.len() + num + 1;  // +1 for ONE
            }
        }

        // Handle bit variables
        if var.contains("_bit_") {
            if let Some(pos) = self.witnesses.iter().position(|x| x == var) {
                return self.pub_inputs.len() + pos + 1;
            }
        }

        // If we get here, the variable is truly unknown
        warn!("Unknown variable: {} (pub_inputs: {:?}, witnesses: {:?}, symbol_map: {:?})", 
              var, self.pub_inputs, self.witnesses, self.symbol_map);
        0 // Return 0 as fallback (ONE variable)
    }

    fn get_next_variable_index(&self) -> usize {
        self.pub_inputs.len() + self.witnesses.len() + self.temp_var_counter + 1 // +1 for ONE
    }
    
    fn resolve_symbol_map_variables(&self, lc: &LinearCombination) -> LinearCombination {
        let mut resolved_terms = Vec::new();
        
        for (var, coeff) in &lc.terms {
            if let Some(substitution) = self.variable_substitutions.get(var) {
                // This variable should be substituted with another linear combination
                for (sub_var, sub_coeff) in &substitution.terms {
                    resolved_terms.push((sub_var.clone(), coeff * sub_coeff));
                }
            } else {
                // Keep the original term
                resolved_terms.push((var.clone(), *coeff));
            }
        }
        
        LinearCombination { terms: resolved_terms }
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
            terms: self.terms.iter()
                .map(|(var, coeff)| (var.clone(), -coeff))
                .collect()
        }
    }
}

impl std::fmt::Display for R1CSConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        for (i, (var, coeff)) in self.a.terms.iter().enumerate() {
            if i > 0 { write!(f, " + ")?; }
            write!(f, "{}*{}", coeff, var)?;
        }
        write!(f, ") * (")?;
        for (i, (var, coeff)) in self.b.terms.iter().enumerate() {
            if i > 0 { write!(f, " + ")?; }
            write!(f, "{}*{}", coeff, var)?;
        }
        write!(f, ") = (")?;
        for (i, (var, coeff)) in self.c.terms.iter().enumerate() {
            if i > 0 { write!(f, " + ")?; }
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
            R1CSError::LinearityViolation(msg) => write!(f, "Linearity violation: {}", msg),
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
            "Invalid magic bytes - not a lof-r1cs file"
        ));
    }
    
    let mut version = [0u8; 4];
    reader.read_exact(&mut version)?;
    if u32::from_le_bytes(version) != 1 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Unsupported r1cs version"
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
        pub_inputs.push(String::from_utf8(name).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?);
    }

    let mut witnesses = Vec::new();
    for _ in 0..witnesses_count {
        reader.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf) as usize;
        let mut name = vec![0u8; len];
        reader.read_exact(&mut name)?;
        witnesses.push(String::from_utf8(name).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?);
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
        
        let mut coeff_buf = [0u8; 8];
        reader.read_exact(&mut coeff_buf)?;
        let coeff = i64::from_le_bytes(coeff_buf);
        
        terms.push((format!("var_{}", var_idx), coeff));
    }
    
    Ok(LinearCombination { terms })
}