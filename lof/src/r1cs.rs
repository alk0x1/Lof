use std::{collections::HashMap, io::Write, path::PathBuf};
use crate::{ast::{self, Constraint, Expression, Operator}, logger::CompilerLogger};
use std::fmt;

#[derive(Debug)]
pub enum R1CSError {
  UnsupportedOperation(String),
  InvalidFunction(String),
  InvalidArgument(String),
  NonQuadratic,
  InvalidExpression,
  FileError,
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

pub struct R1CSGenerator<'a> {
  pub constraints: Vec<R1CSConstraint>,
  pub temp_var_counter: usize,
  pub symbol_map: HashMap<String, usize>,
  pub pub_inputs: Vec<String>,
  pub witnesses: Vec<String>,
  logger: &'a CompilerLogger,
}

impl<'a> R1CSGenerator<'a> {
  pub fn new(logger: &'a CompilerLogger) -> Self {
    Self {
      constraints: Vec::new(),
      temp_var_counter: 0,
      symbol_map: HashMap::new(),
      pub_inputs: Vec::new(),
      witnesses: Vec::new(),
      logger,
    }
  }
    pub fn write_r1cs_file(&self, source_path: &PathBuf) -> std::io::Result<u64> {
  
    let parent_dir = source_path.parent()
      .ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::Other, 
        "Could not determine parent directory"
      ))?;

    let file_stem = source_path.file_stem()
      .ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::Other,
        "Could not get file name"
      ))?;
    
    let mut r1cs_path = parent_dir.to_path_buf();
    r1cs_path.push(file_stem);
    r1cs_path.set_extension("r1cs");
    
    self.logger.writing_r1cs(&r1cs_path);

    let file = match std::fs::File::create(&r1cs_path) {
      Ok(f) => f,
      Err(e) => {
        self.logger.r1cs_write_failed(&e);
        return Err(e);
      }
    };
    
    let mut writer = std::io::BufWriter::new(file);

    writer.write_all(b"r1cs")?;
    
    self.logger.r1cs_metadata(
      self.pub_inputs.len(),
      self.witnesses.len(),
      self.constraints.len()
    );

    writer.write_all(&(self.pub_inputs.len() as u32).to_le_bytes())?;
    writer.write_all(&(self.witnesses.len() as u32).to_le_bytes())?;
    writer.write_all(&(self.constraints.len() as u32).to_le_bytes())?;

    let mut constraints_written = 0;
    for constraint in &self.constraints {
      self.write_linear_combination(&mut writer, &constraint.a)?;
      self.write_linear_combination(&mut writer, &constraint.b)?;
      self.write_linear_combination(&mut writer, &constraint.c)?;
      constraints_written += 1;
    }

    let metadata = std::fs::metadata(&r1cs_path)?;
    self.logger.r1cs_write_success(&r1cs_path, metadata.len(), constraints_written);

    Ok(metadata.len())
}


  fn write_linear_combination<W: Write>(&self, writer: &mut W, lc: &LinearCombination) -> std::io::Result<()> {
    writer.write_all(&(lc.terms.len() as u32).to_le_bytes())?;
    
    for (_, coeff) in &lc.terms {
      writer.write_all(&(coeff).to_le_bytes())?;
      // TODO: Write variable index
      // For now, let's just write a placeholder
      writer.write_all(&0u32.to_le_bytes())?;
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
      Expression::Proof { signals, constraints, .. } => {
        for signal in signals {
          match signal.visibility {
            ast::Visibility::Input |   
            ast::Visibility::Output => {
              self.pub_inputs.push(signal.name.clone());
            }
            ast::Visibility::Witness => {
              self.witnesses.push(signal.name.clone());
            }
          }
        }

        for constraint in constraints {
          self.convert_constraint(constraint)?;
        }

        Ok(())
      }
      _ => Err(R1CSError::InvalidArgument("Expected proof expression".to_string()))
    }
  }

  fn convert_constraint(&mut self, constraint: &Constraint) -> Result<(), R1CSError> {
    match constraint {
      Constraint::Assert(expr) | Constraint::Verify(expr) => {
        self.convert_assertion(expr)?;
      }
      _ => return Err(R1CSError::UnsupportedOperation("Unsupported constraint type".to_string()))
    }
    Ok(())
  }

  fn convert_assertion(&mut self, expr: &Expression) -> Result<(), R1CSError> {
    match expr {
      Expression::BinaryOp { left, op: Operator::Assert, right } => {
        // Convert a === b into a - b = 0
        let a = self.convert_to_linear_combination(left)?;
        let b = self.convert_to_linear_combination(right)?;
        
        // Create a - b = 0 constraint
        let neg_b = b.negate();
        
        self.constraints.push(R1CSConstraint {
          a,
          b: LinearCombination { terms: vec![("ONE".to_string(), 1)] },
          c: neg_b
        });
        
        Ok(())
      }
      _ => Err(R1CSError::InvalidArgument("Expected assertion".to_string()))
    }
  }

  fn convert_to_linear_combination(&mut self, expr: &Expression) -> Result<LinearCombination, R1CSError> {
    match expr {
      Expression::Variable(name) => {
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
        match op {
          Operator::Add => {
            let mut lc = self.convert_to_linear_combination(left)?;
            let rc = self.convert_to_linear_combination(right)?;
            lc.add(&rc);
            Ok(lc)
          }
          Operator::Mul => {
            // Multiplication needs a new constraint and temp variable
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
          _ => Err(R1CSError::UnsupportedOperation(format!("Unsupported operator: {:?}", op)))
        }
      }
      Expression::FunctionCall { function, arguments } => {
        match function.as_str() {
          "decompose" => self.convert_decompose(arguments),
          _ => Err(R1CSError::InvalidFunction(format!("Unsupported function: {}", function)))
        }
      }
      _ => Err(R1CSError::InvalidArgument("Unsupported expression type".to_string()))
    }
  }

  fn convert_decompose(&mut self, arguments: &[Expression]) -> Result<LinearCombination, R1CSError> {
    if arguments.len() != 1 {
      return Err(R1CSError::InvalidArgument("decompose expects one argument".to_string()));
    }

    let bits = match &arguments[0] {
      Expression::Variable(name) => name,
      _ => return Err(R1CSError::InvalidArgument("decompose expects a variable".to_string()))
    };

    // Create constraints for bit decomposition
    // Each bit needs to be 0 or 1: bi * (1 - bi) = 0
    let mut sum_terms = Vec::new();
    for i in 0..8 {
      let bit = format!("{}_bit_{}", bits, i);
      self.witnesses.push(bit.clone());
      
      // bi * (1 - bi) = 0 constraint
      self.constraints.push(R1CSConstraint {
        a: LinearCombination { terms: vec![(bit.clone(), 1)] },
        b: LinearCombination { 
          terms: vec![("ONE".to_string(), 1), (bit.clone(), -1)] 
        },
        c: LinearCombination { terms: vec![] }
      });

      // Add to sum with power of 2
      sum_terms.push((bit, 1 << i));
    }

    Ok(LinearCombination { terms: sum_terms })
  }

  pub fn get_matrices(&self) -> (Vec<Vec<i64>>, Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let n_vars = self.pub_inputs.len() + self.witnesses.len() + self.temp_var_counter;
    let n_constraints = self.constraints.len();

    let mut a_matrix = vec![vec![0; n_vars]; n_constraints];
    let mut b_matrix = vec![vec![0; n_vars]; n_constraints];
    let mut c_matrix = vec![vec![0; n_vars]; n_constraints];

    for (i, constraint) in self.constraints.iter().enumerate() {
      for (var, coeff) in &constraint.a.terms {
        let var_idx = self.get_variable_index(var);
        a_matrix[i][var_idx] = *coeff;
      }

      for (var, coeff) in &constraint.b.terms {
        let var_idx = self.get_variable_index(var);
        b_matrix[i][var_idx] = *coeff;
      }

      for (var, coeff) in &constraint.c.terms {
        let var_idx = self.get_variable_index(var);
        c_matrix[i][var_idx] = *coeff;
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

    panic!("Unknown variable: {}", var)
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
    for (var, coeff) in &self.a.terms {
      write!(f, "{}*{} + ", coeff, var)?;
    }
    write!(f, ") * (")?;
    for (var, coeff) in &self.b.terms {
      write!(f, "{}*{} + ", coeff, var)?;
    }
    write!(f, ") = (")?;
    for (var, coeff) in &self.c.terms {
      write!(f, "{}*{} + ", coeff, var)?;
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
    }
  }
}
