use std::{collections::HashMap, io::{Read, Seek, Write}, path::PathBuf};
use crate::ast::{Expression, Operator, Pattern, Signal, Visibility, Type, LinearityKind};
use tracing::{info, warn, error, debug};
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
    /// Variables that have been consumed (for linearity tracking)
    pub consumed_vars: std::collections::HashSet<String>,
    /// Current scope variables
    pub variables: HashMap<String, Type>,
}

pub struct R1CSGenerator {
    pub constraints: Vec<R1CSConstraint>,
    pub temp_var_counter: usize,
    pub symbol_map: HashMap<String, usize>,
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
            pub_inputs: Vec::new(),
            witnesses: Vec::new(),
            context: R1CSContext {
                consumed_vars: std::collections::HashSet::new(),
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
                // Check if variable is consumed
                if self.context.consumed_vars.contains(name) {
                    return Err(R1CSError::LinearityViolation(
                        format!("Variable '{}' has already been consumed", name)
                    ));
                }
                
                // For linear variables, mark as consumed
                if let Some(var_type) = self.context.variables.get(name) {
                    match var_type {
                        Type::Field(LinearityKind::Linear) | Type::Bool(LinearityKind::Linear) => {
                            self.context.consumed_vars.insert(name.clone());
                        }
                        _ => {} // Copyable variables don't get consumed
                    }
                }
                
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
                // Assert creates a constraint that the condition equals 1
                let cond_lc = self.convert_to_linear_combination(condition)?;
                
                self.constraints.push(R1CSConstraint {
                    a: cond_lc,
                    b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                    c: LinearCombination { terms: vec![("ONE".to_string(), 1)] }
                });
                
                Ok(LinearCombination { terms: vec![] })
            }
            
            Expression::Let { pattern, value, body } => {
                self.convert_let_binding(pattern, value, body)
            }
            
            Expression::Match { value, patterns } => {
                self.convert_match_expression(value, patterns)
            }
            
            Expression::Block { statements, final_expr } => {
                // Create new scope
                let saved_context = self.context.clone();
                
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
                
                // Restore context (block scoping)
                self.context = saved_context;
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
            
            Expression::Dup(expr) => {
                // Dup makes a linear variable copyable
                let _lc = self.convert_to_linear_combination(expr)?;
                // The duplication logic would be handled at type level
                // For R1CS, we just return the same linear combination
                self.convert_to_linear_combination(expr)
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
                
                self.constraints.push(R1CSConstraint {
                    a,
                    b,
                    c: LinearCombination {
                        terms: vec![(temp.clone(), 1)]
                    }
                });
                
                Ok(LinearCombination {
                    terms: vec![(temp, 1)]
                })
            }
            
            Operator::Assert => {
                // Assert operator: left === right
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;

                self.constraints.push(R1CSConstraint {
                    a: left_lc,
                    b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                    c: right_lc
                });
                
                Ok(LinearCombination { terms: vec![] })
            }
            
            Operator::Equal | Operator::NotEqual => {
                // Comparison operations - need special handling for R1CS
                // For now, create a boolean result
                let temp = self.new_temp_var();
                let left_lc = self.convert_to_linear_combination(left)?;
                let right_lc = self.convert_to_linear_combination(right)?;
                
                // Create constraint for equality check
                // TODO: This is simplified a real implementation would need more complex logic
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
              // 1. Simple assignment: let x = expr
              // 2. Constraint generation for computed values
              
              if self.is_simple_variable_or_constant(&value_lc) {
                  // If the value is just a variable or constant, we can alias it
                  debug!("Simple assignment: {} = {:?}", name, value_lc);
                  
                  // Add to context without creating a constraint
                  self.context.variables.insert(name.clone(), Type::Field(LinearityKind::Linear));
                  
                  // Create an alias mapping - the variable points to the same linear combination
                  // We still need a constraint for R1CS completeness
                  self.constraints.push(R1CSConstraint {
                      a: LinearCombination { terms: vec![(name.clone(), 1)] },
                      b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
                      c: value_lc
                  });
              } else {
                  // Complex expression - need to create a witness variable and constraint
                  debug!("Complex assignment: {} = {:?}", name, value_lc);
                  
                  // Add as witness if not already a public input
                  if !self.pub_inputs.contains(name) && !self.witnesses.contains(name) {
                      debug!("Adding {} as witness variable", name);
                      self.witnesses.push(name.clone());
                  }
                  
                  self.context.variables.insert(name.clone(), Type::Field(LinearityKind::Linear));
                  
                  // Create constraint: name * 1 = value_lc
                  debug!("Creating constraint: {} * 1 = {:?}", name, value_lc);
                  self.constraints.push(R1CSConstraint {
                      a: LinearCombination { terms: vec![(name.clone(), 1)] },
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
              // TODO: This is a simplified implementation, full implementation would need
              // to handle tuple decomposition properly
              
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
                          
                          self.context.variables.insert(element_name.clone(), Type::Field(LinearityKind::Linear));
                          
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
      
      // Restore the previous context (lexical scoping)
      // This ensures that variables bound in the let don't leak to outer scope
      debug!("Restoring context after let binding");
      self.context = saved_context;
      
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
        if arguments.len() != 1 {
            return Err(R1CSError::InvalidArgument("decompose expects one argument".to_string()));
        }

        let input_var = match &arguments[0] {
            Expression::Variable(name) => name,
            _ => return Err(R1CSError::InvalidArgument("decompose expects a variable".to_string()))
        };

        let mut sum_terms = Vec::new();
        
        // Create bit variables and constraints
        for i in 0..8 {
            let bit = format!("{}_bit_{}", input_var, i);
            self.witnesses.push(bit.clone());
            
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

        if let Some(pos) = self.pub_inputs.iter().position(|x| x == var) {
            return pos + 1;  // +1 because ONE is at 0
        }

        if let Some(pos) = self.witnesses.iter().position(|x| x == var) {
            return self.pub_inputs.len() + pos + 1;  // +1 for ONE
        }

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

        warn!("Unknown variable: {}", var);
        0 // Return 0 as fallback (ONE variable)
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
        pub_inputs,
        witnesses,
        context: R1CSContext {
            consumed_vars: std::collections::HashSet::new(),
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