use crate::frontend::ast::{Expression, Type, Parameter, Operator};

#[derive(Debug, Clone)]
pub struct R1CS {
  pub num_vars: usize,          // Total number of variables
  pub num_inputs: usize,        // Number of public inputs
  pub num_aux: usize,          // Number of private inputs and intermediate variables
  pub constraints: Vec<Constraint>,
}

#[derive(Debug, Clone)]
pub struct Constraint {
  pub a: Vec<(usize, i64)>,    // Linear combination for left side
  pub b: Vec<(usize, i64)>,    // Linear combination for right side
  pub c: Vec<(usize, i64)>,    // Linear combination for output
}

impl R1CS {
  pub fn new() -> Self {
    R1CS {
      num_vars: 0,
      num_inputs: 0,
      num_aux: 0,
      constraints: Vec::new(),
    }
  }

  pub fn from_ast(expr: &Expression) -> Result<Self, String> {
    let mut r1cs = R1CS::new();
    r1cs.process_expression(expr)?;
    Ok(r1cs)
  }

  fn process_expression(&mut self, expr: &Expression) -> Result<usize, String> {
    match expr {
      Expression::Theorem { name: _, inputs: _, output: _, body } => {
        // For a theorem, we just process its body
        self.process_expression(body)
      },
      Expression::Number(n) => {
        // Constants get their own variable
        let var_idx = self.num_vars;
        self.num_vars += 1;
        self.num_aux += 1;
        // Add constraint: var = n
        self.constraints.push(Constraint {
          a: vec![(0, 1)],  // 1
          b: vec![(0, 1)],  // 1
          c: vec![(var_idx, 1), (0, -*n)],  // var - n
        });
        Ok(var_idx)
      },
      Expression::Variable(_) => {
        // For now, just allocate a new variable
        let var_idx = self.num_vars;
        self.num_vars += 1;
        self.num_inputs += 1;  // Assuming all variables are inputs for now
        Ok(var_idx)
      },
      Expression::BinaryOp { left, operator, right } => {
        let left_idx = self.process_expression(left)?;
        let right_idx = self.process_expression(right)?;
        
        match operator {
          Operator::Multiply => {
            // For multiplication: z = x * y
            let result_idx = self.num_vars;
            self.num_vars += 1;
            self.num_aux += 1;
            
            self.constraints.push(Constraint {
              a: vec![(left_idx, 1)],
              b: vec![(right_idx, 1)],
              c: vec![(result_idx, 1)],
            });
            
            Ok(result_idx)
          },
          Operator::Add => {
            // For addition: z = x + y
            let result_idx = self.num_vars;
            self.num_vars += 1;
            self.num_aux += 1;
            
            self.constraints.push(Constraint {
              a: vec![(left_idx, 1), (right_idx, 1)],
              b: vec![(0, 1)],  // Multiply by 1
              c: vec![(result_idx, 1)],
            });
            
            Ok(result_idx)
          },
          Operator::Equals => {
            // For equality: left - right = 0
            let result_idx = self.num_vars;
            self.num_vars += 1;
            self.num_aux += 1;
            
            self.constraints.push(Constraint {
              a: vec![(left_idx, 1), (right_idx, -1)],  // left - right
              b: vec![(0, 1)],  // * 1
              c: vec![(0, 0)],  // = 0
            });
            
            Ok(result_idx)
          },
          _ => Err("Unsupported operator in R1CS generation".to_string()),
        }
      },
      _ => Err("Unsupported expression type in R1CS generation".to_string()),
    }
  }
}