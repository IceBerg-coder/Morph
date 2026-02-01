use crate::ast::*;
use crate::interpreter::value::Value;
use super::{Type, TypeEnvironment, TypeError, GhostAttribute, GhostValue, annotation_to_type};
use regex::Regex;

/// Type checker for Morph programs
pub struct TypeChecker {
    environment: TypeEnvironment,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            environment: TypeEnvironment::new(),
            errors: Vec::new(),
        }
    }

    /// Check a complete module
    pub fn check_module(&mut self, module: &Module) -> Result<(), Vec<TypeError>> {
        // First pass: register all type declarations
        for decl in &module.declarations {
            if let Declaration::Type(type_decl) = decl {
                if let Err(e) = self.register_type_declaration(type_decl) {
                    self.errors.push(e);
                }
            }
        }

        // Second pass: register all function signatures
        for decl in &module.declarations {
            if let Declaration::Function(func) = decl {
                if let Err(e) = self.register_function_signature(func) {
                    self.errors.push(e);
                }
            }
        }

        // Third pass: type check function bodies
        for decl in &module.declarations {
            match decl {
                Declaration::Function(func) => {
                    if let Err(e) = self.check_function(func) {
                        self.errors.push(e);
                    }
                }
                Declaration::Solve(solve) => {
                    if let Err(e) = self.check_solve_block(solve) {
                        self.errors.push(e);
                    }
                }
                _ => {}
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Register a type declaration
    fn register_type_declaration(&mut self, decl: &TypeDecl) -> Result<(), TypeError> {
        let ty = match &decl.definition {
            TypeDefinition::Alias(annotation) => {
                annotation_to_type(annotation, &self.environment)?
            }
            TypeDefinition::Record(fields) => {
                let mut field_types = std::collections::HashMap::new();
                for (name, annotation) in fields {
                    field_types.insert(name.clone(), annotation_to_type(annotation, &self.environment)?);
                }
                Type::Record(field_types)
            }
            TypeDefinition::Enum(variants) => {
                // For now, enums are treated as strings
                Type::String
            }
        };
        
        self.environment.define_type(decl.name.clone(), ty);
        Ok(())
    }

    /// Register a function signature
    fn register_function_signature(&mut self, func: &FunctionDecl) -> Result<(), TypeError> {
        let param_types: Result<Vec<_>, _> = func.params
            .iter()
            .map(|p| {
                if let Some(ref annotation) = p.type_annotation {
                    annotation_to_type(annotation, &self.environment)
                } else {
                    Ok(Type::Variable(format!("param_{}", p.name)))
                }
            })
            .collect();
        
        let return_type = if let Some(ref annotation) = func.return_type {
            annotation_to_type(annotation, &self.environment)?
        } else {
            Type::Unit
        };
        
        let func_type = Type::Function(param_types?, Box::new(return_type));
        self.environment.define_variable(func.name.clone(), func_type);
        
        Ok(())
    }

    /// Type check a function
    fn check_function(&mut self, func: &FunctionDecl) -> Result<(), TypeError> {
        // Create new scope for function
        let previous = self.environment.clone();
        self.environment = TypeEnvironment::with_parent(self.environment.clone());
        
        // Bind parameters
        for param in &func.params {
            let param_type = if let Some(ref annotation) = param.type_annotation {
                annotation_to_type(annotation, &previous)?
            } else {
                Type::Variable(format!("param_{}", param.name))
            };
            self.environment.define_variable(param.name.clone(), param_type);
        }
        
        // Get expected return type
        let expected_return = if let Some(ref annotation) = func.return_type {
            annotation_to_type(annotation, &previous)?
        } else {
            Type::Unit
        };
        
        // Check function body
        for stmt in &func.body {
            self.check_statement(stmt)?;
        }
        
        // Restore environment
        self.environment = previous;
        
        Ok(())
    }

    /// Type check a solve block
    fn check_solve_block(&mut self, solve: &SolveBlock) -> Result<(), TypeError> {
        // Create new scope
        let previous = self.environment.clone();
        self.environment = TypeEnvironment::with_parent(self.environment.clone());
        
        // Bind parameters
        for param in &solve.params {
            let param_type = if let Some(ref annotation) = param.type_annotation {
                annotation_to_type(annotation, &previous)?
            } else {
                Type::Variable(format!("param_{}", param.name))
            };
            self.environment.define_variable(param.name.clone(), param_type);
        }
        
        // Check constraints
        for constraint in &solve.constraints {
            match constraint {
                Constraint::Binding { name, expr } => {
                    let ty = self.infer_expression(expr)?;
                    self.environment.define_variable(name.clone(), ty);
                }
                Constraint::Ensure(expr) => {
                    let ty = self.infer_expression(expr)?;
                    if ty != Type::Bool {
                        return Err(TypeError::Mismatch {
                            expected: Type::Bool,
                            got: ty,
                        });
                    }
                }
            }
        }
        
        // Restore environment
        self.environment = previous;
        
        Ok(())
    }

    /// Type check a statement
    fn check_statement(&mut self, stmt: &Statement) -> Result<(), TypeError> {
        match stmt {
            Statement::VariableDecl { name, type_annotation, initializer, .. } => {
                let inferred = self.infer_expression(initializer)?;
                
                // If type annotation provided, check compatibility
                if let Some(ref annotation) = type_annotation {
                    let annotated = annotation_to_type(annotation, &self.environment)?;
                    if !self.is_compatible(&inferred, &annotated) {
                        return Err(TypeError::Mismatch {
                            expected: annotated,
                            got: inferred,
                        });
                    }
                    self.environment.define_variable(name.clone(), annotated);
                } else {
                    self.environment.define_variable(name.clone(), inferred);
                }
                Ok(())
            }
            Statement::Expression(expr) => {
                self.infer_expression(expr)?;
                Ok(())
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.infer_expression(expr)?;
                }
                Ok(())
            }
            Statement::For { variable, iterable, guard, body } => {
                let iter_type = self.infer_expression(iterable)?;
                let element_type = match iter_type {
                    Type::List(elem) => *elem,
                    _ => return Err(TypeError::Custom(
                        format!("For loop requires a list, got {:?}", iter_type)
                    )),
                };
                
                // Create new scope for loop
                let previous = self.environment.clone();
                self.environment = TypeEnvironment::with_parent(self.environment.clone());
                
                self.environment.define_variable(variable.clone(), element_type);
                
                // Check guard if present
                if let Some(guard_expr) = guard {
                    let guard_type = self.infer_expression(guard_expr)?;
                    if guard_type != Type::Bool {
                        return Err(TypeError::Mismatch {
                            expected: Type::Bool,
                            got: guard_type,
                        });
                    }
                }
                
                // Check body
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                
                // Restore environment
                self.environment = previous;
                Ok(())
            }
            Statement::Assignment { target, value } => {
                // For now, simple assignment checking
                self.infer_expression(value)?;
                Ok(())
            }
        }
    }

    /// Infer the type of an expression
    fn infer_expression(&mut self, expr: &Expression) -> Result<Type, TypeError> {
        match expr {
            Expression::Literal(lit) => self.infer_literal(lit),
            Expression::Identifier(name) => {
                // Check for built-in functions first
                match name.as_str() {
                    "print" | "log" => {
                        // Built-in print/log functions accept any arguments and return Unit
                        return Ok(Type::Function(
                            vec![Type::Variable("args".to_string())],
                            Box::new(Type::Unit)
                        ));
                    }
                    "len" => {
                        return Ok(Type::Function(
                            vec![Type::Variable("collection".to_string())],
                            Box::new(Type::Int)
                        ));
                    }
                    "range" => {
                        return Ok(Type::Function(
                            vec![Type::Int, Type::Int],
                            Box::new(Type::List(Box::new(Type::Int)))
                        ));
                    }
                    "sqrt" => {
                        return Ok(Type::Function(
                            vec![Type::Float],
                            Box::new(Type::Float)
                        ));
                    }
                    _ => {}
                }
                self.environment.get_variable(name)
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))
            }
            Expression::Binary { left, op, right } => {
                let left_type = self.infer_expression(left)?;
                let right_type = self.infer_expression(right)?;
                self.infer_binary_op(&left_type, op, &right_type)
            }
            Expression::Unary { op, expr } => {
                let expr_type = self.infer_expression(expr)?;
                self.infer_unary_op(op, &expr_type)
            }
            Expression::Call { callee, args } => {
                let callee_type = self.infer_expression(callee)?;
                let arg_types: Result<Vec<_>, _> = args
                    .iter()
                    .map(|a| self.infer_expression(a))
                    .collect();
                
                match callee_type {
                    Type::Function(params, ret) => {
                        let arg_types = arg_types?;
                        if params.len() != arg_types.len() {
                            return Err(TypeError::ArityMismatch {
                                expected: params.len(),
                                got: arg_types.len(),
                            });
                        }
                        Ok(*ret)
                    }
                    _ => Err(TypeError::Custom("Not a function".to_string())),
                }
            }
            Expression::Pipe { left, right } => {
                // For now, treat pipe as function call
                self.infer_expression(right)
            }
            Expression::Match { expr, arms } => {
                let _match_type = self.infer_expression(expr)?;
                // Infer type from first arm
                if let Some(first_arm) = arms.first() {
                    self.infer_expression(&first_arm.expr)
                } else {
                    Ok(Type::Unit)
                }
            }
            Expression::Block(stmts) => {
                let previous = self.environment.clone();
                self.environment = TypeEnvironment::with_parent(self.environment.clone());
                
                let mut result = Type::Unit;
                for stmt in stmts {
                    if let Statement::Expression(expr) = stmt {
                        result = self.infer_expression(expr)?;
                    } else {
                        self.check_statement(stmt)?;
                    }
                }
                
                self.environment = previous;
                Ok(result)
            }
            Expression::If { condition, then_branch, else_branch } => {
                let cond_type = self.infer_expression(condition)?;
                if cond_type != Type::Bool {
                    return Err(TypeError::Mismatch {
                        expected: Type::Bool,
                        got: cond_type,
                    });
                }
                
                let then_type = self.infer_expression(then_branch)?;
                if let Some(else_expr) = else_branch {
                    let else_type = self.infer_expression(else_expr)?;
                    // For now, require exact match
                    if then_type != else_type {
                        return Err(TypeError::Mismatch {
                            expected: then_type,
                            got: else_type,
                        });
                    }
                }
                
                Ok(then_type)
            }
            Expression::FieldAccess { object, field } => {
                let obj_type = self.infer_expression(object)?;
                match obj_type {
                    Type::Record(fields) => {
                        fields.get(field)
                            .cloned()
                            .ok_or_else(|| TypeError::Custom(
                                format!("Field '{}' not found", field)
                            ))
                    }
                    _ => Err(TypeError::Custom("Not a record".to_string())),
                }
            }
            Expression::IndexAccess { object, index } => {
                let obj_type = self.infer_expression(object)?;
                let idx_type = self.infer_expression(index)?;
                
                if idx_type != Type::Int {
                    return Err(TypeError::Mismatch {
                        expected: Type::Int,
                        got: idx_type,
                    });
                }
                
                match obj_type {
                    Type::List(elem) => Ok(*elem),
                    Type::String => Ok(Type::String),
                    _ => Err(TypeError::Custom("Not indexable".to_string())),
                }
            }
            Expression::Lambda { params, body } => {
                // Create new scope
                let previous = self.environment.clone();
                self.environment = TypeEnvironment::with_parent(self.environment.clone());
                
                let mut param_types = Vec::new();
                for param in params {
                    let param_type = if let Some(ref annotation) = param.type_annotation {
                        annotation_to_type(annotation, &previous)?
                    } else {
                        Type::Variable(format!("param_{}", param.name))
                    };
                    self.environment.define_variable(param.name.clone(), param_type.clone());
                    param_types.push(param_type);
                }
                
                let ret_type = self.infer_expression(body)?;
                
                self.environment = previous;
                Ok(Type::Function(param_types, Box::new(ret_type)))
            }
            Expression::Claim(expr) => {
                self.infer_expression(expr)
            }
        }
    }

    /// Infer type of a literal
    fn infer_literal(&self, lit: &Literal) -> Result<Type, TypeError> {
        match lit {
            Literal::Integer(_) => Ok(Type::Int),
            Literal::Float(_) => Ok(Type::Float),
            Literal::String(_) => Ok(Type::String),
            Literal::Boolean(_) => Ok(Type::Bool),
            Literal::List(items) => {
                if items.is_empty() {
                    Ok(Type::List(Box::new(Type::Variable("a".to_string()))))
                } else {
                    // Infer from first element
                    // For now, return generic list
                    Ok(Type::List(Box::new(Type::Variable("a".to_string()))))
                }
            }
            Literal::Record(_) => {
                // For now, return generic record
                Ok(Type::Record(std::collections::HashMap::new()))
            }
        }
    }

    /// Infer type of binary operation
    fn infer_binary_op(&self, left: &Type, op: &BinaryOp, right: &Type) -> Result<Type, TypeError> {
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                match (left, right) {
                    (Type::Int, Type::Int) => Ok(Type::Int),
                    (Type::Float, Type::Float) => Ok(Type::Float),
                    (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(Type::Float),
                    (Type::String, Type::String) if *op == BinaryOp::Add => Ok(Type::String),
                    // Allow operations with type variables (for polymorphic functions)
                    (Type::Variable(_), Type::Int) | (Type::Int, Type::Variable(_)) => Ok(Type::Int),
                    (Type::Variable(_), Type::Float) | (Type::Float, Type::Variable(_)) => Ok(Type::Float),
                    (Type::Variable(_), Type::String) | (Type::String, Type::Variable(_)) if *op == BinaryOp::Add => Ok(Type::String),
                    (Type::Variable(_), Type::Variable(_)) => Ok(Type::Variable("result".to_string())),
                    _ => Err(TypeError::InvalidOperation(
                        format!("Cannot {:?} {:?} and {:?}", op, left, right)
                    )),
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEq | BinaryOp::Greater | BinaryOp::GreaterEq => {
                Ok(Type::Bool)
            }
        }
    }

    /// Infer type of unary operation
    fn infer_unary_op(&self, op: &UnaryOp, expr: &Type) -> Result<Type, TypeError> {
        match op {
            UnaryOp::Negate => {
                match expr {
                    Type::Int => Ok(Type::Int),
                    Type::Float => Ok(Type::Float),
                    _ => Err(TypeError::InvalidOperation(
                        format!("Cannot negate {:?}", expr)
                    )),
                }
            }
            UnaryOp::Not => Ok(Type::Bool),
        }
    }

    /// Check if two types are compatible
    fn is_compatible(&self, inferred: &Type, annotated: &Type) -> bool {
        match (inferred, annotated) {
            (Type::Int, Type::Float) => true, // Int can be used where Float expected
            (a, b) => a == b,
        }
    }
}

/// Validate a value against Ghost type constraints (runtime validation in proto mode)
pub fn validate_ghost_type(value: &Value, ghost_attrs: &[GhostAttribute]) -> Result<(), TypeError> {
    for attr in ghost_attrs {
        match attr.key.as_str() {
            "Regex" => {
                if let GhostValue::String(pattern) = &attr.value {
                    if let Value::String(s) = value {
                        let regex = Regex::new(pattern)
                            .map_err(|e| TypeError::GhostValidationFailed {
                                type_name: "String".to_string(),
                                reason: format!("Invalid regex pattern: {}", e),
                            })?;
                        if !regex.is_match(s) {
                            return Err(TypeError::GhostValidationFailed {
                                type_name: "String".to_string(),
                                reason: format!("Value '{}' does not match pattern '{}'", s, pattern),
                            });
                        }
                    }
                }
            }
            "Min" => {
                if let GhostValue::Number(min) = &attr.value {
                    match value {
                        Value::Integer(n) if (*n as f64) < *min => {
                            return Err(TypeError::GhostValidationFailed {
                                type_name: "Int".to_string(),
                                reason: format!("Value {} is less than minimum {}", n, min),
                            });
                        }
                        Value::Float(n) if *n < *min => {
                            return Err(TypeError::GhostValidationFailed {
                                type_name: "Float".to_string(),
                                reason: format!("Value {} is less than minimum {}", n, min),
                            });
                        }
                        _ => {}
                    }
                }
            }
            "Max" => {
                if let GhostValue::Number(max) = &attr.value {
                    match value {
                        Value::Integer(n) if (*n as f64) > *max => {
                            return Err(TypeError::GhostValidationFailed {
                                type_name: "Int".to_string(),
                                reason: format!("Value {} is greater than maximum {}", n, max),
                            });
                        }
                        Value::Float(n) if *n > *max => {
                            return Err(TypeError::GhostValidationFailed {
                                type_name: "Float".to_string(),
                                reason: format!("Value {} is greater than maximum {}", n, max),
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {} // Unknown ghost attributes are ignored
        }
    }
    
    Ok(())
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}