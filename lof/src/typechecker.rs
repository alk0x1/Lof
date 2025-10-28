use crate::ast::{ConstraintStatus, Expression, Operator, Pattern, Refinement, Type, Visibility};
use std::collections::{HashMap, HashSet};
use std::fmt;

pub struct TypeChecker {
    symbols: HashMap<String, Type>,
    witnesses: HashSet<String>,
    dependencies: HashMap<String, HashSet<String>>,
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
    InvalidExpression,
    UnconstrainedWitness { name: String, witness_type: Type },
    NonZeroRequired { found: Type },
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
            TypeError::EmptyMatchExpression => {
                write!(f, "Match expression must have at least one pattern")
            }
            TypeError::DuplicatePatternVariable(name) => write!(
                f,
                "Variable '{}' is bound multiple times in the same pattern",
                name
            ),
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
            TypeError::UnconstrainedWitness { name, witness_type } => {
                write!(
                    f,
                    "Unconstrained witness '{}' of type {}\n\
                     \nUnconstrained witnesses allow malicious provers to forge proofs.\n\
                     \nHelp: Use '{}' in a constraint:\n\
                     - Multiplication:  let result = {} * other\n\
                     - Assertion:       assert {} > 0\n\
                     - Match:           match {} with ...",
                    name, witness_type, name, name, name, name
                )
            }
            TypeError::NonZeroRequired { found } => write!(
                f,
                "Division requires NonZero<field>, found {}\n\
                 \nDivision by zero creates undefined behavior in circuits.\n\
                 \nHelp: Prove the denominator is non-zero:\n\
                 assert denominator != 0;",
                found
            ),
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbols: HashMap::new(),
            witnesses: HashSet::new(),
            dependencies: HashMap::new(),
        }
    }

    fn field_type(constraint: ConstraintStatus, refinement: Option<Refinement>) -> Type {
        Type::Field {
            constraint,
            refinement,
        }
    }

    fn bool_type(constraint: ConstraintStatus) -> Type {
        Type::Bool { constraint }
    }

    fn is_field_type(typ: &Type) -> bool {
        matches!(typ, Type::Field { .. })
    }

    fn is_bool_type(typ: &Type) -> bool {
        matches!(typ, Type::Bool { .. })
    }

    fn is_numeric_type(typ: &Type) -> bool {
        matches!(typ, Type::Field { .. } | Type::Bool { .. })
    }

    fn promote_to_constrained_direct(&mut self, var_name: &str) {
        if let Some(typ) = self.symbols.get_mut(var_name) {
            match typ {
                Type::Field { constraint, .. } => {
                    *constraint = ConstraintStatus::Constrained;
                }
                Type::Bool { constraint } => {
                    *constraint = ConstraintStatus::Constrained;
                }
                _ => {}
            }
        }
    }

    fn promote_to_constrained(&mut self, var_name: &str) {
        if let Some(deps) = self.dependencies.get(var_name).cloned() {
            for dep in deps {
                self.promote_to_constrained(&dep);
            }
        }

        if let Some(typ) = self.symbols.get_mut(var_name) {
            match typ {
                Type::Field { constraint, .. } => {
                    *constraint = ConstraintStatus::Constrained;
                }
                Type::Bool { constraint } => {
                    *constraint = ConstraintStatus::Constrained;
                }
                _ => {}
            }
        }
    }

    #[allow(dead_code, clippy::collapsible_match)]
    fn promote_to_nonzero(&mut self, var_name: &str) {
        if let Some(deps) = self.dependencies.get(var_name).cloned() {
            for dep in deps {
                self.promote_to_nonzero(&dep);
            }
        }

        if let Some(typ) = self.symbols.get_mut(var_name) {
            if let Type::Field {
                constraint,
                refinement,
            } = typ
            {
                *constraint = ConstraintStatus::Constrained;
                *refinement = Some(Refinement::NonZero);
            }
        }
    }

    fn promote_expression_to_nonzero(&mut self, expr: &Expression) {
        if let Expression::Variable(name) = expr {
            self.promote_to_nonzero(name);
        }
    }

    fn mark_nonzero_from_assert(&mut self, condition: &Expression) {
        if let Expression::BinaryOp { left, op, right } = condition {
            if *op == Operator::NotEqual {
                if Self::is_zero_literal(left) {
                    self.promote_expression_to_nonzero(right);
                } else if Self::is_zero_literal(right) {
                    self.promote_expression_to_nonzero(left);
                }
            }
        }
    }

    fn ensure_nonzero_field(&self, expr: &Expression, typ: &Type) -> Result<(), TypeError> {
        if let Expression::Number(value) = expr {
            if *value == 0 {
                return Err(TypeError::NonZeroRequired { found: typ.clone() });
            }
            return Ok(());
        }

        match typ {
            Type::Field {
                refinement: Some(Refinement::NonZero),
                ..
            } => Ok(()),
            Type::Bool { .. } => Err(TypeError::NonZeroRequired { found: typ.clone() }),
            _ => Err(TypeError::NonZeroRequired { found: typ.clone() }),
        }
    }

    fn is_zero_literal(expr: &Expression) -> bool {
        matches!(expr, Expression::Number(n) if *n == 0)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_vars(&self, expr: &Expression, vars: &mut HashSet<String>) {
        match expr {
            Expression::Variable(name) => {
                vars.insert(name.clone());
            }
            Expression::BinaryOp { left, right, .. } => {
                self.extract_vars(left, vars);
                self.extract_vars(right, vars);
            }
            Expression::Tuple(elements) => {
                for elem in elements {
                    self.extract_vars(elem, vars);
                }
            }
            Expression::ArrayIndex { array, index } => {
                self.extract_vars(array, vars);
                self.extract_vars(index, vars);
            }
            _ => {}
        }
    }

    fn read_variable(&self, name: &str) -> Result<Type, TypeError> {
        let var_type = self
            .symbols
            .get(name)
            .cloned()
            .ok_or_else(|| TypeError::UndefinedVariable(name.to_string()))?;

        Ok(var_type)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn check_pattern_compatibility(&self, pattern: &Pattern, typ: &Type) -> Result<(), TypeError> {
        match (pattern, typ) {
            (Pattern::Variable(_), _) | (Pattern::Wildcard, _) => Ok(()),
            (Pattern::Literal(_), Type::Field { .. }) => Ok(()),
            (Pattern::Tuple(patterns), Type::Tuple(types)) => {
                if patterns.len() != types.len() {
                    return Err(TypeError::PatternMismatch {
                        expected: typ.clone(),
                        found: pattern.clone(),
                    });
                }
                for (p, t) in patterns.iter().zip(types.iter()) {
                    self.check_pattern_compatibility(p, t)?;
                }
                Ok(())
            }
            _ => Err(TypeError::PatternMismatch {
                expected: typ.clone(),
                found: pattern.clone(),
            }),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn check_pattern_duplicates(
        &self,
        pattern: &Pattern,
        bound_vars: &mut HashSet<String>,
    ) -> Result<(), TypeError> {
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

    #[allow(clippy::only_used_in_recursion)]
    fn types_compatible(&self, type1: &Type, type2: &Type) -> bool {
        match (type1, type2) {
            (Type::Field { .. }, Type::Field { .. }) => true,
            (Type::Bool { .. }, Type::Bool { .. }) => true,
            (Type::Tuple(t1), Type::Tuple(t2)) => {
                t1.len() == t2.len()
                    && t1
                        .iter()
                        .zip(t2.iter())
                        .all(|(a, b)| self.types_compatible(a, b))
            }
            (
                Type::Array {
                    element_type: e1,
                    size: s1,
                },
                Type::Array {
                    element_type: e2,
                    size: s2,
                },
            ) => s1 == s2 && self.types_compatible(e1, e2),
            (
                Type::Function {
                    params: p1,
                    return_type: r1,
                },
                Type::Function {
                    params: p2,
                    return_type: r2,
                },
            ) => {
                p1.len() == p2.len()
                    && p1
                        .iter()
                        .zip(p2.iter())
                        .all(|(a, b)| self.types_compatible(a, b))
                    && self.types_compatible(r1, r2)
            }
            _ => type1 == type2,
        }
    }

    pub fn check_program(&mut self, program: &[Expression]) -> Result<(), TypeError> {
        for expr in program {
            match expr {
                Expression::FunctionDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
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
                Expression::Component {
                    name,
                    signals,
                    body,
                    ..
                } => {
                    let input_params: Vec<Type> = signals
                        .iter()
                        .filter(|s| s.visibility == Visibility::Input)
                        .map(|s| self.resolve_type(&s.typ))
                        .collect::<Result<Vec<_>, _>>()?;

                    let saved_symbols = self.symbols.clone();

                    for signal in signals {
                        let resolved_type = self.resolve_type(&signal.typ)?;
                        self.symbols.insert(signal.name.clone(), resolved_type);
                    }

                    let return_type = self.check_expression(body)?;

                    self.symbols = saved_symbols;

                    let component_type = if input_params.is_empty() {
                        return_type
                    } else {
                        self.build_curried_function_type(input_params, return_type)
                    };

                    self.symbols.insert(name.clone(), component_type);
                }
                _ => {}
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

    fn apply_function(
        &mut self,
        mut function_type: Type,
        arguments: &[Expression],
    ) -> Result<Type, TypeError> {
        if arguments.is_empty() {
            return Ok(function_type);
        }

        for argument in arguments {
            match function_type {
                Type::Function {
                    params,
                    return_type,
                } => {
                    if params.len() != 1 {
                        return Err(TypeError::ArgumentCountMismatch {
                            expected: 1,
                            found: params.len(),
                        });
                    }

                    let expected_param = params
                        .into_iter()
                        .next()
                        .expect("function type with zero params should be caught above");

                    let argument_type = self.check_expression(argument)?;

                    if !self.types_compatible(&expected_param, &argument_type) {
                        return Err(TypeError::TypeMismatch {
                            expected: expected_param,
                            found: argument_type,
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
            Expression::Number(value) => {
                let refinement = if *value == 0 {
                    None
                } else {
                    Some(Refinement::NonZero)
                };
                Ok(Self::field_type(ConstraintStatus::Constrained, refinement))
            }
            Expression::Variable(name) => self.read_variable(name),
            Expression::Let {
                pattern,
                value,
                body,
            } => {
                let value_type = self.check_expression(value)?;

                let mut bound_variables = Vec::new();
                Self::collect_pattern_variables(pattern, &mut bound_variables);

                let mut previous_symbols = HashMap::new();
                let mut previous_dependencies = HashMap::new();

                for var in &bound_variables {
                    if let Some(existing) = self.symbols.get(var).cloned() {
                        previous_symbols.insert(var.clone(), existing);
                    }
                    if let Some(existing_deps) = self.dependencies.get(var).cloned() {
                        previous_dependencies.insert(var.clone(), existing_deps);
                    }
                }

                match pattern {
                    Pattern::Variable(var_name) => {
                        self.symbols.insert(var_name.clone(), value_type.clone());

                        // track dependencies through let bindings to allows transitive promotion
                        let mut deps = HashSet::new();
                        self.extract_vars(value, &mut deps);
                        if !deps.is_empty() {
                            self.dependencies.insert(var_name.clone(), deps);
                        } else {
                            self.dependencies.remove(var_name);
                        }
                    }
                    _ => {
                        self.bind_pattern(pattern, &value_type)?;
                    }
                }

                let body_type = self.check_expression(body)?;

                for var in bound_variables {
                    if let Some(old_type) = previous_symbols.remove(&var) {
                        self.symbols.insert(var.clone(), old_type);
                    } else {
                        self.symbols.remove(&var);
                    }

                    if let Some(old_deps) = previous_dependencies.remove(&var) {
                        self.dependencies.insert(var.clone(), old_deps);
                    } else {
                        self.dependencies.remove(&var);
                    }
                }

                Ok(body_type)
            }
            Expression::BinaryOp { left, op, right } => {
                let left_type = self.check_expression(left)?;
                let right_type = self.check_expression(right)?;

                if matches!(op, Operator::Mul | Operator::Div | Operator::Assert) {
                    let mut vars = HashSet::new();
                    self.extract_vars(left, &mut vars);
                    self.extract_vars(right, &mut vars);
                    for var in vars {
                        self.promote_to_constrained(&var);
                    }
                }

                if matches!(op, Operator::Div) {
                    self.ensure_nonzero_field(right, &right_type)?;
                }

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
                if !Self::is_bool_type(&cond_type) {
                    return Err(TypeError::NonBooleanInAssert(cond_type));
                }

                // promote all variables in the condition to constrained
                // for constraint equality (===), promote transitively because they create R1CS constraints
                // for comparison assertions (>, <, ==, etc.), only promote direct variables
                let use_transitive = matches!(
                    condition.as_ref(),
                    Expression::BinaryOp {
                        op: Operator::Assert,
                        ..
                    }
                );

                let mut vars = HashSet::new();
                self.extract_vars(condition, &mut vars);
                for var in vars {
                    if use_transitive {
                        self.promote_to_constrained(&var);
                    } else {
                        self.promote_to_constrained_direct(&var);
                    }
                }

                self.mark_nonzero_from_assert(condition);

                Ok(Type::Unit)
            }
            Expression::FunctionDef {
                name: _,
                body,
                params,
                return_type,
            } => {
                let original_symbols = self.symbols.clone();

                for param in params.iter() {
                    let resolved_param_type = self.resolve_type(&param.typ)?;
                    self.symbols.insert(param.name.clone(), resolved_param_type);
                }

                let body_type = self.check_expression(body)?;
                self.symbols = original_symbols;

                let expected_return_type = self.resolve_type(return_type)?;

                // use compatibility check instead of exact equality
                // this allows field^constrained to match field^unconstrained in return types
                if !self.types_compatible(&body_type, &expected_return_type) {
                    return Err(TypeError::TypeMismatch {
                        expected: expected_return_type,
                        found: body_type,
                    });
                }
                Ok(Type::Unit)
            }
            Expression::FunctionCall {
                function,
                arguments,
            } => {
                let function_type = self
                    .symbols
                    .get(function)
                    .cloned()
                    .ok_or_else(|| TypeError::UndefinedFunction(function.clone()))?;

                self.apply_function(function_type, arguments)
            }
            Expression::Proof { signals, body, .. } => {
                let saved_symbols = self.symbols.clone();
                let saved_dependencies = self.dependencies.clone();
                let saved_witnesses = std::mem::take(&mut self.witnesses);

                let result = (|| -> Result<Type, TypeError> {
                    for signal in signals {
                        let resolved_type = self.resolve_type(&signal.typ)?;

                        // inputs are inherently constrained (they're public)
                        let final_type = if signal.visibility == Visibility::Input {
                            match resolved_type {
                                Type::Field { refinement, .. } => {
                                    Self::field_type(ConstraintStatus::Constrained, refinement)
                                }
                                Type::Bool { .. } => Self::bool_type(ConstraintStatus::Constrained),
                                other => other,
                            }
                        } else {
                            resolved_type
                        };

                        self.symbols.insert(signal.name.clone(), final_type);

                        if signal.visibility == Visibility::Witness {
                            self.witnesses.insert(signal.name.clone());
                        }
                    }

                    let body_type = self.check_expression(body)?;

                    // validate all witnesses are constrained
                    for witness_name in &self.witnesses {
                        if let Some(typ) = self.symbols.get(witness_name) {
                            let is_unconstrained = matches!(
                                typ,
                                Type::Field {
                                    constraint: ConstraintStatus::Unconstrained,
                                    ..
                                } | Type::Bool {
                                    constraint: ConstraintStatus::Unconstrained
                                }
                            );

                            if is_unconstrained {
                                return Err(TypeError::UnconstrainedWitness {
                                    name: witness_name.clone(),
                                    witness_type: typ.clone(),
                                });
                            }
                        }
                    }

                    let mut body_vars = HashSet::new();
                    self.extract_vars(body, &mut body_vars);

                    for var_name in &body_vars {
                        if let Some(typ) = self.symbols.get(var_name) {
                            let is_unconstrained = matches!(
                                typ,
                                Type::Field {
                                    constraint: ConstraintStatus::Unconstrained,
                                    ..
                                } | Type::Bool {
                                    constraint: ConstraintStatus::Unconstrained
                                }
                            );

                            if is_unconstrained {
                                return Err(TypeError::UnconstrainedWitness {
                                    name: var_name.clone(),
                                    witness_type: typ.clone(),
                                });
                            }
                        }
                    }

                    match body_type {
                        Type::Field {
                            constraint: ConstraintStatus::Unconstrained,
                            ..
                        }
                        | Type::Bool {
                            constraint: ConstraintStatus::Unconstrained,
                        } => {
                            return Err(TypeError::UnconstrainedWitness {
                                name: "<proof body result>".to_string(),
                                witness_type: body_type,
                            });
                        }
                        _ => {}
                    }

                    Ok(Type::Unit)
                })();

                self.symbols = saved_symbols;
                self.dependencies = saved_dependencies;
                self.witnesses = saved_witnesses;

                result
            }
            Expression::Block {
                statements,
                final_expr,
            } => {
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

                let mut vars = HashSet::new();
                self.extract_vars(value, &mut vars);
                for var in vars {
                    self.promote_to_constrained(&var);
                }

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
                        if !self.types_compatible(first_type, arm_type) {
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
            Expression::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    return Err(TypeError::InvalidExpression);
                }

                let first_type = self.check_expression(&elements[0])?;
                for elem in elements.iter().skip(1) {
                    let elem_type = self.check_expression(elem)?;
                    if !self.types_compatible(&first_type, &elem_type) {
                        return Err(TypeError::TypeMismatch {
                            expected: first_type.clone(),
                            found: elem_type,
                        });
                    }
                }

                Ok(Type::Array {
                    element_type: Box::new(first_type),
                    size: elements.len(),
                })
            }

            Expression::ArrayIndex { array, index } => {
                let array_type = self.check_expression(array)?;
                let index_type = self.check_expression(index)?;

                if !Self::is_field_type(&index_type) {
                    return Err(TypeError::TypeMismatch {
                        expected: Self::field_type(ConstraintStatus::Constrained, None),
                        found: index_type,
                    });
                }

                if let Expression::Variable(var_name) = index.as_ref() {
                    self.promote_to_constrained(var_name);
                }

                match array_type {
                    Type::Array { element_type, .. } => Ok(*element_type),
                    _ => Err(TypeError::TypeMismatch {
                        expected: Type::Array {
                            element_type: Box::new(Self::field_type(
                                ConstraintStatus::Constrained,
                                None,
                            )),
                            size: 0,
                        },
                        found: array_type,
                    }),
                }
            }

            Expression::TypeAlias { .. } => Ok(Type::Unit),

            Expression::EnumDef { .. } => Ok(Type::Unit),

            Expression::Component { signals, body, .. } => {
                let mut bound_names = Vec::new();
                let mut previous_symbols = HashMap::new();
                let mut previous_dependencies = HashMap::new();

                for signal in signals {
                    if let Some(existing) = self.symbols.get(&signal.name).cloned() {
                        previous_symbols.insert(signal.name.clone(), existing);
                    }
                    if let Some(existing_deps) = self.dependencies.get(&signal.name).cloned() {
                        previous_dependencies.insert(signal.name.clone(), existing_deps);
                    }

                    let resolved_type = self.resolve_type(&signal.typ)?;
                    self.symbols.insert(signal.name.clone(), resolved_type);
                    bound_names.push(signal.name.clone());
                }

                let _body_type = self.check_expression(body)?;

                for name in bound_names {
                    if let Some(old_type) = previous_symbols.remove(&name) {
                        self.symbols.insert(name.clone(), old_type);
                    } else {
                        self.symbols.remove(&name);
                    }

                    if let Some(old_deps) = previous_dependencies.remove(&name) {
                        self.dependencies.insert(name.clone(), old_deps);
                    } else {
                        self.dependencies.remove(&name);
                    }
                }

                Ok(Type::Unit)
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn resolve_type(&self, typ: &Type) -> Result<Type, TypeError> {
        match typ {
            Type::Identifier(name) => match name.as_str() {
                "field" => Ok(Self::field_type(ConstraintStatus::Unconstrained, None)),
                "bool" => Ok(Self::bool_type(ConstraintStatus::Unconstrained)),
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
            Type::Refined(base_type, _predicate) => self.resolve_type(base_type),
            _ => Ok(typ.clone()),
        }
    }

    fn collect_pattern_variables(pattern: &Pattern, vars: &mut Vec<String>) {
        match pattern {
            Pattern::Variable(name) => vars.push(name.clone()),
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    Self::collect_pattern_variables(p, vars);
                }
            }
            Pattern::Constructor(_, patterns) => {
                for p in patterns {
                    Self::collect_pattern_variables(p, vars);
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
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
            _ => Err(TypeError::PatternMismatch {
                expected: typ.clone(),
                found: pattern.clone(),
            }),
        }
    }

    fn check_operator(
        &mut self,
        op: &Operator,
        left: &Type,
        right: &Type,
    ) -> Result<Type, TypeError> {
        match op {
            Operator::Add | Operator::Sub => {
                let left_numeric = Self::is_numeric_type(left);
                let right_numeric = Self::is_numeric_type(right);

                if left_numeric && right_numeric {
                    Ok(Self::field_type(ConstraintStatus::Unconstrained, None))
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: Self::field_type(ConstraintStatus::Constrained, None),
                        found: if !left_numeric {
                            left.clone()
                        } else {
                            right.clone()
                        },
                    })
                }
            }

            Operator::Mul | Operator::Div => {
                let left_numeric = Self::is_numeric_type(left);
                let right_numeric = Self::is_numeric_type(right);
                let denominator_valid = if matches!(op, Operator::Div) {
                    Self::is_field_type(right)
                } else {
                    true
                };

                if left_numeric && right_numeric && denominator_valid {
                    Ok(Self::field_type(ConstraintStatus::Constrained, None))
                } else {
                    let offending = if !left_numeric {
                        left.clone()
                    } else {
                        right.clone()
                    };
                    Err(TypeError::TypeMismatch {
                        expected: Self::field_type(ConstraintStatus::Constrained, None),
                        found: offending,
                    })
                }
            }

            Operator::Equal | Operator::NotEqual => {
                if self.types_compatible(left, right) {
                    Ok(Self::bool_type(ConstraintStatus::Unconstrained))
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: left.clone(),
                        found: right.clone(),
                    })
                }
            }

            Operator::Lt | Operator::Gt | Operator::Le | Operator::Ge => {
                let left_is_field = Self::is_field_type(left);
                let right_is_field = Self::is_field_type(right);

                if left_is_field && right_is_field {
                    Ok(Self::bool_type(ConstraintStatus::Unconstrained))
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: Self::field_type(ConstraintStatus::Constrained, None),
                        found: if !left_is_field {
                            left.clone()
                        } else {
                            right.clone()
                        },
                    })
                }
            }

            Operator::And | Operator::Or => {
                if Self::is_bool_type(left) && Self::is_bool_type(right) {
                    Ok(Self::bool_type(ConstraintStatus::Unconstrained))
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: Self::bool_type(ConstraintStatus::Constrained),
                        found: if !Self::is_bool_type(left) {
                            left.clone()
                        } else {
                            right.clone()
                        },
                    })
                }
            }

            Operator::Not => {
                if Self::is_bool_type(right) {
                    Ok(Self::bool_type(ConstraintStatus::Unconstrained))
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: Self::bool_type(ConstraintStatus::Constrained),
                        found: right.clone(),
                    })
                }
            }

            Operator::Assert => {
                let compatible = self.types_compatible(left, right)
                    || (Self::is_bool_type(left) && Self::is_field_type(right))
                    || (Self::is_field_type(left) && Self::is_bool_type(right));

                if compatible {
                    Ok(Self::bool_type(ConstraintStatus::Constrained))
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: left.clone(),
                        found: right.clone(),
                    })
                }
            }
        }
    }
}
