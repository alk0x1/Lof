use crate::{ast, ast::{Constraint, Expression, Operator, Signal, Type}};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug)]
pub enum TypeError {
  UndefinedVariable(String),
  TypeMismatch { expected: Type, found: Type },
  InvalidOperator { op: Operator, found: Type },
  UnusedVariable(String),
  IncompletePatterns,
  NonTerminatingRecursion,
  RangeConstraintViolation,
  ResourceUsageError(String),
  SoundnessError(String),
  NoPublicSignals,
  NoConstraints,
  NonQuadraticConstraint(Box<Expression>),
  UnusedWitness(String),
  CircularDependency(String),
  UndefinedBeforeUse(String),
  NonLinearUsage(String),
  DegreeViolation {
    expr: Box<Expression>,
    degree: u32,
  },
  UnconstrainedPath,
}

impl fmt::Display for TypeError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      TypeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
      TypeError::TypeMismatch { expected, found } => 
        write!(f, "Type mismatch: expected {:?}, found {:?}", expected, found),
      TypeError::InvalidOperator { op, found } => 
        write!(f, "Invalid operator {:?} for type {:?}", op, found),
      TypeError::UnusedVariable(name) => write!(f, "Unused variable: {}", name),
      TypeError::IncompletePatterns => write!(f, "Incomplete pattern matching"),
      TypeError::NonTerminatingRecursion => write!(f, "Non-terminating recursion detected"),
      TypeError::RangeConstraintViolation => write!(f, "Range constraint violation"),
      TypeError::ResourceUsageError(msg) => write!(f, "Resource usage error: {}", msg),
      TypeError::SoundnessError(msg) => write!(f, "Soundness error: {}", msg),
      TypeError::NoPublicSignals => write!(f, "No public signals defined"),
      TypeError::NoConstraints => write!(f, "No constraints defined"),
      TypeError::NonQuadraticConstraint(expr) => write!(f, "Non-quadratic constraint: {:?}", expr),
      TypeError::UnusedWitness(name) => write!(f, "Unused witness: {}", name),
      TypeError::CircularDependency(cycle) => write!(f, "Circular dependency: {}", cycle),
      TypeError::UndefinedBeforeUse(name) => write!(f, "Variable used before definition: {}", name),
      TypeError::NonLinearUsage(name) => write!(f, "Non-linear usage of variable: {}", name),
      TypeError::DegreeViolation { expr, degree } => 
        write!(f, "Degree violation: expression {:?} has degree {}", expr, degree),
      TypeError::UnconstrainedPath => write!(f, "Unconstrained execution path"),
    }
  }
}

pub struct TypeChecker {
  // Symbol table for variables and their types
  symbols: HashMap<String, Type>,
  // Track resource usage
  used_vars: HashMap<String, usize>,           // Linear Types
  // New fields for enhanced checking
  dependencies: HashMap<String, Vec<String>>,  // Track variable dependencies
  degrees: HashMap<String, u32>,              // Track polynomial degrees (Dependent Types)
  constrained_paths: bool,                    // Track if all paths lead to constraints
  public_signals: usize,                      // Count of public signals
  constraint_count: usize,                    // Count of constraints
}

impl TypeChecker {
  pub fn new() -> Self {
    TypeChecker {
      symbols: HashMap::new(),
      used_vars: HashMap::new(),
      dependencies: HashMap::new(),
      degrees: HashMap::new(),
      constrained_paths: false,
      public_signals: 0,
      constraint_count: 0,
    }
  }

  pub fn check_proof(&mut self, expr: &Expression) -> Result<Type, TypeError> {
    match expr {
      Expression::Proof { signals, constraints, .. } => {
        self.check_basic_structure(signals, constraints)?;
        self.collect_signals(signals)?;
        self.build_dependency_graph(constraints)?;
        self.verify_constraints(constraints)?;
        self.verify_resource_usage()?;
        self.verify_path_constraints(expr)?;
        
        Ok(Type::Unit)
      }
      _ => Err(TypeError::SoundnessError("Expected proof".to_string()))
    }
  }

  fn check_basic_structure(&mut self, signals: &[Signal], constraints: &[Constraint]) -> Result<(), TypeError> {
    let has_public = signals.iter().any(|s| 
      matches!(s.visibility, ast::Visibility::Input | ast::Visibility::Output)
    );
    if !has_public {
      return Err(TypeError::NoPublicSignals);
    }

    if constraints.is_empty() {
      return Err(TypeError::NoConstraints);
    }

    Ok(())
  }

  fn build_dependency_graph(&mut self, constraints: &[Constraint]) -> Result<(), TypeError> {
    for constraint in constraints {
      match constraint {
        Constraint::Assert(expr) | Constraint::Verify(expr) => {
          self.collect_dependencies(expr)?;
        }
        _ => {}
      }
    }
    self.check_circular_dependencies()?;
    
    Ok(())
  }

  fn collect_dependencies(&mut self, expr: &Expression) -> Result<Vec<String>, TypeError> {
    match expr {
      Expression::Variable(name) => {
        Ok(vec![name.clone()])
      }
      Expression::BinaryOp { left, right, .. } => {
        let mut deps = self.collect_dependencies(left)?;
        deps.extend(self.collect_dependencies(right)?);
        Ok(deps)
      }
      _ => Ok(vec![])
    }
  }

  fn check_circular_dependencies(&self) -> Result<(), TypeError> {
    let mut index = 0;
    let mut stack: Vec<String> = Vec::new();
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut lowlinks: HashMap<String, usize> = HashMap::new();
    let mut on_stack: HashSet<String> = HashSet::new();

    // Helper function for Tarjan's algorithm
    fn strongconnect(
      v: &str,
      index: &mut usize,
      stack: &mut Vec<String>,
      indices: &mut HashMap<String, usize>,
      lowlinks: &mut HashMap<String, usize>,
      on_stack: &mut HashSet<String>,
      dependencies: &HashMap<String, Vec<String>>,
    ) -> Result<(), TypeError> {
      indices.insert(v.to_string(), *index);
      lowlinks.insert(v.to_string(), *index);
      *index += 1;
      stack.push(v.to_string());
      on_stack.insert(v.to_string());

      // Consider successors of v
      if let Some(deps) = dependencies.get(v) {
        for &ref w in deps {
          if !indices.contains_key(w) {
            // Successor w has not yet been visited; recurse on it
            strongconnect(w, index, stack, indices, lowlinks, on_stack, dependencies)?;
            let v_lowlink = lowlinks.get(v).unwrap();
            let w_lowlink = lowlinks.get(w).unwrap();
            lowlinks.insert(v.to_string(), std::cmp::min(*v_lowlink, *w_lowlink));
          } else if on_stack.contains(w) {
            // Successor w is in stack and hence in the current SCC
            let v_lowlink = lowlinks.get(v).unwrap();
            let w_index = indices.get(w).unwrap();
            lowlinks.insert(v.to_string(), std::cmp::min(*v_lowlink, *w_index));
          }
        }
      }

        // If v is a root node, pop the stack and generate an SCC
        if let Some(v_lowlink) = lowlinks.get(v) {
          if let Some(v_index) = indices.get(v) {
            if v_lowlink == v_index {
              // Found a strongly connected component
              let mut cycle = Vec::new();
              loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                cycle.push(w.clone());
                if w == v {
                  break;
                }
              }
              if cycle.len() > 1 {
                return Err(TypeError::CircularDependency(
                  cycle.join(" -> ")
                ));
              }
            }
          }
        }

        Ok(())
    }

    // Run Tarjan's algorithm from each unvisited node
    for var in self.dependencies.keys() {
      if !indices.contains_key(var) {
        strongconnect(
          var,
          &mut index,
          &mut stack,
          &mut indices,
          &mut lowlinks,
          &mut on_stack,
          &self.dependencies,
        )?;
      }
    }

    Ok(())
  }

  fn verify_constraints(&mut self, constraints: &[Constraint]) -> Result<(), TypeError> {
    for constraint in constraints {
        self.check_constraint(constraint)?;

        match constraint {
          Constraint::Assert(expr) | Constraint::Verify(expr) => {
            let degree = self.calculate_degree(expr)?;
            if degree > 2 {
              return Err(TypeError::DegreeViolation {
                expr: expr.clone(),
                degree,
              });
            }
            self.constraint_count += 1;
          }
          _ => {}
        }
      }
      Ok(())
  }

  fn calculate_degree(&self, expr: &Expression) -> Result<u32, TypeError> {
    match expr {
      Expression::Variable(_) => Ok(1),
      Expression::Number(_) => Ok(0),
      Expression::BinaryOp { left, op, right } => {
        let left_degree = self.calculate_degree(left)?;
        let right_degree = self.calculate_degree(right)?;
        match op {
          Operator::Mul => Ok(left_degree + right_degree),
          Operator::Add | Operator::Sub => Ok(std::cmp::max(left_degree, right_degree)),
          _ => Ok(std::cmp::max(left_degree, right_degree))
        }
      }
      _ => Ok(0)
    }
  }

  fn verify_path_constraints(&mut self, expr: &Expression) -> Result<(), TypeError> {
    match expr {
      Expression::Proof { constraints, .. } => {
        for constraint in constraints {
          match constraint {
            Constraint::Assert(_) | Constraint::Verify(_) => {
              self.constrained_paths = true;
              break;
            }
            _ => {}
          }
        }
        if !self.constrained_paths {
          return Err(TypeError::UnconstrainedPath);
        }
        
        Ok(())
      }
      _ => Ok(())
    }
  }

  fn collect_signals(&mut self, signals: &[Signal]) -> Result<(), TypeError> {
    for signal in signals {
      self.symbols.insert(signal.name.clone(), signal.typ.clone());
      self.used_vars.insert(signal.name.clone(), 0);
    }
    Ok(())
  }

  fn check_constraint(&mut self, constraint: &Constraint) -> Result<Type, TypeError> {
    match constraint {
      Constraint::Assert(expr) => {
        let typ = self.check_expression(expr)?;
        match typ {
          Type::Bool | Type::Field => Ok(Type::Unit),
          _ => Err(TypeError::TypeMismatch {
            expected: Type::Bool,
            found: typ
          })
        }
      },
      Constraint::Verify(expr) => {
        let typ = self.check_expression(expr)?;
        match typ {
          Type::Bool | Type::Field => Ok(Type::Unit),
          _ => Err(TypeError::TypeMismatch {
            expected: Type::Bool,
            found: typ
          })
        }
      },
      Constraint::Let(expr) => self.check_expression(expr),
      Constraint::Match(expr) => self.check_expression(expr),
    }
  }

  fn check_expression(&mut self, expr: &Expression) -> Result<Type, TypeError> {
    match expr {
      Expression::Variable(name) => {
        *self.used_vars.entry(name.clone()).or_insert(0) += 1;
        
        self.symbols.get(name)
          .cloned()
          .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))
      }
      Expression::BinaryOp { left, op, right } => {
        let left_type = self.check_expression(left)?;
        let right_type = self.check_expression(right)?;
        
        self.check_operator(op.clone(), &left_type, &right_type)
      }
      Expression::FunctionCall { function, arguments } => {
        self.check_function_call(function, arguments)
      }
      _ => todo!("Implement other expression types")
    }
  }

  fn check_operator(&self, op: Operator, left: &Type, right: &Type) -> Result<Type, TypeError> {
    match op {
      Operator::Add | Operator::Mul => {
        match (left, right) {
          (Type::Field, Type::Field) => Ok(Type::Field),
          _ => Err(TypeError::InvalidOperator {
            op,
            found: left.clone()
          })
        }
      }
      Operator::Assert => {
        if left == right {
          Ok(Type::Bool)
        } else {
          Err(TypeError::TypeMismatch {
            expected: left.clone(),
            found: right.clone()
          })
        }
      }
      _ => todo!("Implement other operators")
    }
  }

  fn verify_resource_usage(&self) -> Result<(), TypeError> {
    for (var, count) in &self.used_vars {
      if *count == 0 {
        return Err(TypeError::UnusedVariable(var.clone()));
      }
      if *count > 1 {
        return Err(TypeError::ResourceUsageError(
          format!("Variable {} used {} times", var, count)
        ));
      }
    }
    Ok(())
  }

  fn verify_completeness(&self, expr: &Expression) -> Result<(), TypeError> {
    // Verify pattern matching is exhaustive
    // Verify all paths lead to constraints
    todo!("Implement completeness checking")
  }

  fn check_path_constraints(&mut self, expr: &Expression) -> Result<(), TypeError> {
    match expr {
      Expression::Block(exprs) => {
        for expr in exprs {
          self.check_path_constraints(expr)?;
        }
      }
      Expression::BinaryOp { left, right, op } => {
        if matches!(op, Operator::Assert) {
          self.constrained_paths = true;
        }
        self.check_path_constraints(left)?;
        self.check_path_constraints(right)?;
      }
      _ => {}
    }
    Ok(())
  }

  fn check_function_call(&mut self, function: &str, arguments: &[Expression]) -> Result<Type, TypeError> {
    match function {
      "decompose" => self.check_decompose(arguments),
      _ => Err(TypeError::UndefinedVariable(function.to_string())),
    }
  }

  fn check_decompose(&mut self, arguments: &[Expression]) -> Result<Type, TypeError> {
    if arguments.len() != 1 {
      return Err(TypeError::InvalidOperator { 
        op: Operator::Decompose, 
        found: Type::Field 
      });
    }

    let arg = &arguments[0];
    let arg_type = self.check_expression(arg)?;

    match arg_type {
      Type::Bits(ref size) => {
        // Calculate the range based on bit size
        // For Bits<N>, the range is 0..2^N-1
        match **size {
          Expression::Number(n) => {
            let max = (1 << n) - 1;
            Ok(Type::FieldRange(
              Box::new(Expression::Number(0)),
              Box::new(Expression::Number(max))
            ))
          },
          _ => Err(TypeError::TypeMismatch {
            expected: Type::Bits(Box::new(Expression::Number(8))),
            found: arg_type.clone(),
          })
        }
      }
      _ => Err(TypeError::TypeMismatch {
        expected: Type::Bits(Box::new(Expression::Number(8))),
        found: arg_type,
      })
    }
  }
}
