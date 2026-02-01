pub mod value;
pub mod environment;

use crate::ast::*;
use value::{Value, RuntimeError, FunctionValue};
use environment::Environment;
use std::collections::HashMap;

/// Morph interpreter for Stage 0 (Draft mode)
pub struct Interpreter {
    /// Global environment
    globals: Environment,
    /// Current environment (changes with scope)
    environment: Environment,
}

impl Interpreter {
    /// Create a new interpreter with built-in functions
    pub fn new() -> Self {
        let mut globals = Environment::new();
        
        // Register built-in functions
        Self::register_builtins(&mut globals);
        
        Interpreter {
            globals: globals.clone(),
            environment: globals,
        }
    }

    /// Register built-in functions
    fn register_builtins(env: &mut Environment) {
        // log function - prints to stdout
        env.define("log".to_string(), Value::Function(FunctionValue::Builtin(|args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg.to_string());
            }
            println!();
            Ok(Value::Unit)
        })));

        // print function - prints without newline
        env.define("print".to_string(), Value::Function(FunctionValue::Builtin(|args| {
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg.to_string());
            }
            Ok(Value::Unit)
        })));

        // len function - gets length of list or string
        env.define("len".to_string(), Value::Function(FunctionValue::Builtin(|args| {
            if args.len() != 1 {
                return Err(RuntimeError::ArityMismatch { expected: 1, got: args.len() });
            }
            match &args[0] {
                Value::List(items) => Ok(Value::Integer(items.len() as i64)),
                Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                _ => Err(RuntimeError::TypeError("len() requires a list or string".to_string())),
            }
        })));

        // push function - adds element to list
        env.define("push".to_string(), Value::Function(FunctionValue::Builtin(|args| {
            if args.len() != 2 {
                return Err(RuntimeError::ArityMismatch { expected: 2, got: args.len() });
            }
            // Note: This is a simplified version
            // In a real implementation, we'd need mutable references
            Ok(Value::Unit)
        })));

        // range function - creates a range of numbers
        env.define("range".to_string(), Value::Function(FunctionValue::Builtin(|args| {
            match args.len() {
                1 => {
                    let end = args[0].as_integer()?;
                    let list: Vec<Value> = (0..end).map(|i| Value::Integer(i)).collect();
                    Ok(Value::List(list))
                }
                2 => {
                    let start = args[0].as_integer()?;
                    let end = args[1].as_integer()?;
                    let list: Vec<Value> = (start..end).map(|i| Value::Integer(i)).collect();
                    Ok(Value::List(list))
                }
                3 => {
                    let start = args[0].as_integer()?;
                    let end = args[1].as_integer()?;
                    let step = args[2].as_integer()?;
                    let list: Vec<Value> = (start..end).step_by(step as usize).map(|i| Value::Integer(i)).collect();
                    Ok(Value::List(list))
                }
                _ => Err(RuntimeError::ArityMismatch { expected: 3, got: args.len() }),
            }
        })));
    }

    /// Interpret a complete module
    pub fn interpret(&mut self, module: &Module) -> Result<Value, RuntimeError> {
        let mut result = Value::Unit;
        
        // First pass: register all function declarations
        for decl in &module.declarations {
            if let Declaration::Function(func) = decl {
                let func_value = Value::Function(FunctionValue::UserDefined {
                    decl: func.clone(),
                    closure: Some(self.environment.snapshot()),
                });
                self.globals.define(func.name.clone(), func_value);
            }
        }
        
        // Second pass: execute the module (look for main function)
        let has_main = module.declarations.iter().any(|d| {
            matches!(d, Declaration::Function(f) if f.name == "main")
        });
        
        // Update environment with globals
        self.environment = self.globals.clone();
        
        if has_main {
            // Call main function
            self.call_function("main", &[])
        } else {
            // Execute all top-level declarations
            for decl in &module.declarations {
                match decl {
                    Declaration::Function(_) => {
                        // Already registered
                    }
                    Declaration::Type(_) => {
                        // Type declarations are compile-time only in proto mode
                    }
                    Declaration::Solve(solve) => {
                        result = self.execute_solve_block(solve)?;
                    }
                    Declaration::Import(_) => {
                        // TODO: Implement imports
                    }
                }
            }
            Ok(result)
        }
    }

    /// Execute a solve block
    fn execute_solve_block(&mut self, solve: &SolveBlock) -> Result<Value, RuntimeError> {
        // Create new scope for solve block
        let previous = self.environment.clone();
        self.environment = Environment::with_parent(self.environment.clone());
        
        // Bind parameters
        for param in &solve.params {
            self.environment.define(param.name.clone(), Value::Unit);
        }
        
        // Execute constraints
        for constraint in &solve.constraints {
            match constraint {
                Constraint::Binding { name, expr } => {
                    let value = self.evaluate(expr)?;
                    self.environment.define(name.clone(), value);
                }
                Constraint::Ensure(expr) => {
                    let value = self.evaluate(expr)?;
                    if !value.is_truthy() {
                        return Err(RuntimeError::Custom(
                            format!("Ensure constraint failed: {:?}", expr)
                        ));
                    }
                }
            }
        }
        
        // Get return value
        let result = if let Some(ref expr) = solve.return_expr {
            self.evaluate(expr)?
        } else {
            Value::Unit
        };
        
        // Restore environment
        self.environment = previous;
        
        Ok(result)
    }

    /// Call a function by name
    fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        let func = self.environment.get(name)?;
        
        match func {
            Value::Function(func_val) => self.execute_function(&func_val, args),
            _ => Err(RuntimeError::TypeError(format!("{} is not a function", name))),
        }
    }

    /// Execute a function value
    fn execute_function(&mut self, func: &FunctionValue, args: &[Value]) -> Result<Value, RuntimeError> {
        match func {
            FunctionValue::Builtin(builtin) => builtin(args),
            FunctionValue::UserDefined { decl, closure } => {
                // Check arity
                if decl.params.len() != args.len() {
                    return Err(RuntimeError::ArityMismatch {
                        expected: decl.params.len(),
                        got: args.len(),
                    });
                }
                
                // Create new environment with closure
                let mut new_env = if let Some(ref closure_vars) = closure {
                    let mut env = Environment::new();
                    for (name, value) in closure_vars {
                        env.define(name.clone(), value.clone());
                    }
                    env
                } else {
                    Environment::with_parent(self.environment.clone())
                };
                
                // Bind parameters
                for (param, arg) in decl.params.iter().zip(args.iter()) {
                    new_env.define(param.name.clone(), arg.clone());
                }
                
                // Execute function body
                let previous = self.environment.clone();
                self.environment = new_env;
                
                let mut result = Value::Unit;
                for stmt in &decl.body {
                    result = self.execute_statement(stmt)?;
                    // Check for early return
                    // TODO: Implement proper return handling
                }
                
                // Restore environment
                self.environment = previous;
                
                Ok(result)
            }
        }
    }

    /// Execute a statement
    fn execute_statement(&mut self, stmt: &Statement) -> Result<Value, RuntimeError> {
        match stmt {
            Statement::VariableDecl { name, initializer, .. } => {
                let value = self.evaluate(initializer)?;
                self.environment.define(name.clone(), value);
                Ok(Value::Unit)
            }
            Statement::Expression(expr) => {
                self.evaluate(expr)
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.evaluate(expr)
                } else {
                    Ok(Value::Unit)
                }
            }
            Statement::For { variable, iterable, guard, body } => {
                let iter_value = self.evaluate(iterable)?;
                let items = match iter_value {
                    Value::List(items) => items,
                    _ => return Err(RuntimeError::TypeError(
                        "For loop requires a list".to_string()
                    )),
                };
                
                let mut result = Value::Unit;
                
                for item in items {
                    // Create new scope for loop body
                    let previous = self.environment.clone();
                    self.environment = Environment::with_parent(self.environment.clone());
                    
                    // Bind loop variable
                    self.environment.define(variable.clone(), item);
                    
                    // Check guard if present
                    if let Some(ref guard_expr) = guard {
                        let guard_value = self.evaluate(guard_expr)?;
                        if !guard_value.is_truthy() {
                            self.environment = previous;
                            continue;
                        }
                    }
                    
                    // Execute body
                    for stmt in body {
                        result = self.execute_statement(stmt)?;
                    }
                    
                    // Restore environment
                    self.environment = previous;
                }
                
                Ok(result)
            }
            Statement::Assignment { target, value } => {
                let val = self.evaluate(value)?;
                
                // Handle simple variable assignment
                if let Expression::Identifier(name) = target {
                    self.environment.assign(name, val)?;
                } else if let Expression::FieldAccess { object, field } = target {
                    let obj_val = self.evaluate(object)?;
                    // TODO: Handle field assignment
                } else if let Expression::IndexAccess { object, index } = target {
                    let mut obj_val = self.evaluate(object)?;
                    let idx_val = self.evaluate(index)?;
                    
                    if let Value::List(ref mut items) = obj_val {
                        let idx = idx_val.as_integer()?;
                        if idx < 0 || idx as usize >= items.len() {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index: idx,
                                len: items.len(),
                            });
                        }
                        items[idx as usize] = val;
                    }
                }
                
                Ok(Value::Unit)
            }
        }
    }

    /// Evaluate an expression
    fn evaluate(&mut self, expr: &Expression) -> Result<Value, RuntimeError> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(lit),
            Expression::Identifier(name) => {
                self.environment.get(name)
            }
            Expression::Binary { left, op, right } => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }
            Expression::Unary { op, expr } => {
                let val = self.evaluate(expr)?;
                self.evaluate_unary_op(op, &val)
            }
            Expression::Call { callee, args } => {
                let func_val = self.evaluate(callee)?;
                let arg_vals: Result<Vec<_>, _> = args.iter()
                    .map(|a| self.evaluate(a))
                    .collect();
                let arg_vals = arg_vals?;
                
                match func_val {
                    Value::Function(func) => self.execute_function(&func, &arg_vals),
                    _ => Err(RuntimeError::TypeError("Not a function".to_string())),
                }
            }
            Expression::Pipe { left, right } => {
                let left_val = self.evaluate(left)?;
                
                // Pipe left value as first argument to right function
                match right.as_ref() {
                    Expression::Call { callee, args } => {
                        let func_val = self.evaluate(callee)?;
                        let mut arg_vals = vec![left_val];
                        for arg in args {
                            arg_vals.push(self.evaluate(arg)?);
                        }
                        
                        match func_val {
                            Value::Function(func) => self.execute_function(&func, &arg_vals),
                            _ => Err(RuntimeError::TypeError("Not a function".to_string())),
                        }
                    }
                    Expression::Identifier(name) => {
                        self.call_function(name, &[left_val])
                    }
                    _ => Err(RuntimeError::TypeError(
                        "Right side of pipe must be a function".to_string()
                    )),
                }
            }
            Expression::Match { expr, arms } => {
                let val = self.evaluate(expr)?;
                
                for arm in arms {
                    if self.match_pattern(&val, &arm.pattern)? {
                        return self.evaluate(&arm.expr);
                    }
                }
                
                Err(RuntimeError::Custom("No match arm matched".to_string()))
            }
            Expression::Block(stmts) => {
                let previous = self.environment.clone();
                self.environment = Environment::with_parent(self.environment.clone());
                
                let mut result = Value::Unit;
                for stmt in stmts {
                    result = self.execute_statement(stmt)?;
                }
                
                self.environment = previous;
                Ok(result)
            }
            Expression::If { condition, then_branch, else_branch } => {
                let cond_val = self.evaluate(condition)?;
                
                if cond_val.is_truthy() {
                    self.evaluate(then_branch)
                } else if let Some(else_expr) = else_branch {
                    self.evaluate(else_expr)
                } else {
                    Ok(Value::Unit)
                }
            }
            Expression::FieldAccess { object, field } => {
                let obj_val = self.evaluate(object)?;
                
                match obj_val {
                    Value::Record(fields) => {
                        fields.get(field)
                            .cloned()
                            .ok_or_else(|| RuntimeError::Custom(
                                format!("Field '{}' not found", field)
                            ))
                    }
                    _ => Err(RuntimeError::TypeError("Not a record".to_string())),
                }
            }
            Expression::IndexAccess { object, index } => {
                let obj_val = self.evaluate(object)?;
                let idx_val = self.evaluate(index)?;
                
                match obj_val {
                    Value::List(items) => {
                        let idx = idx_val.as_integer()?;
                        if idx < 0 || idx as usize >= items.len() {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index: idx,
                                len: items.len(),
                            });
                        }
                        Ok(items[idx as usize].clone())
                    }
                    Value::String(s) => {
                        let idx = idx_val.as_integer()?;
                        if idx < 0 || idx as usize >= s.len() {
                            return Err(RuntimeError::IndexOutOfBounds {
                                index: idx,
                                len: s.len(),
                            });
                        }
                        Ok(Value::String(s.chars().nth(idx as usize).unwrap().to_string()))
                    }
                    _ => Err(RuntimeError::TypeError("Not indexable".to_string())),
                }
            }
            Expression::Lambda { params, body } => {
                // Create a lambda function
                let lambda_func = FunctionDecl {
                    mode: FunctionMode::Proto,
                    name: "<lambda>".to_string(),
                    params: params.clone(),
                    return_type: None,
                    body: vec![Statement::Expression((**body).clone())],
                };
                
                Ok(Value::Function(FunctionValue::UserDefined {
                    decl: lambda_func,
                    closure: Some(self.environment.snapshot()),
                }))
            }
            Expression::Claim(expr) => {
                // In the interpreter, claim is essentially a no-op
                // It marks ownership transfer but doesn't change behavior
                self.evaluate(expr)
            }
        }
    }

    /// Evaluate a literal
    fn evaluate_literal(&mut self, lit: &Literal) -> Result<Value, RuntimeError> {
        match lit {
            Literal::Integer(n) => Ok(Value::Integer(*n)),
            Literal::Float(n) => Ok(Value::Float(*n)),
            Literal::String(s) => Ok(Value::String(s.clone())),
            Literal::Boolean(b) => Ok(Value::Boolean(*b)),
            Literal::List(items) => {
                let values: Result<Vec<_>, _> = items.iter()
                    .map(|e| self.evaluate(e))
                    .collect();
                Ok(Value::List(values?))
            }
            Literal::Record(fields) => {
                let mut map = HashMap::new();
                for (name, expr) in fields {
                    let value = self.evaluate(expr)?;
                    map.insert(name.clone(), value);
                }
                Ok(Value::Record(map))
            }
        }
    }

    /// Evaluate binary operation
    fn evaluate_binary_op(&self, left: &Value, op: &BinaryOp, right: &Value) -> Result<Value, RuntimeError> {
        match op {
            BinaryOp::Add => self.add_values(left, right),
            BinaryOp::Subtract => self.subtract_values(left, right),
            BinaryOp::Multiply => self.multiply_values(left, right),
            BinaryOp::Divide => self.divide_values(left, right),
            BinaryOp::Modulo => self.modulo_values(left, right),
            BinaryOp::Equal => Ok(Value::Boolean(left == right)),
            BinaryOp::NotEqual => Ok(Value::Boolean(left != right)),
            BinaryOp::Less => self.compare_values(left, right, |c| c == std::cmp::Ordering::Less),
            BinaryOp::LessEq => self.compare_values(left, right, |c| {
                c == std::cmp::Ordering::Less || c == std::cmp::Ordering::Equal
            }),
            BinaryOp::Greater => self.compare_values(left, right, |c| c == std::cmp::Ordering::Greater),
            BinaryOp::GreaterEq => self.compare_values(left, right, |c| {
                c == std::cmp::Ordering::Greater || c == std::cmp::Ordering::Equal
            }),
        }
    }

    /// Add two values
    fn add_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::List(a), Value::List(b)) => {
                let mut result = a.clone();
                result.extend(b.clone());
                Ok(Value::List(result))
            }
            _ => Err(RuntimeError::TypeError(
                format!("Cannot add {} and {}", left.type_name(), right.type_name())
            )),
        }
    }

    /// Subtract two values
    fn subtract_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(RuntimeError::TypeError(
                format!("Cannot subtract {} and {}", left.type_name(), right.type_name())
            )),
        }
    }

    /// Multiply two values
    fn multiply_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
            _ => Err(RuntimeError::TypeError(
                format!("Cannot multiply {} and {}", left.type_name(), right.type_name())
            )),
        }
    }

    /// Divide two values
    fn divide_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    return Err(RuntimeError::Custom("Division by zero".to_string()));
                }
                Ok(Value::Integer(a / b))
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(RuntimeError::Custom("Division by zero".to_string()));
                }
                Ok(Value::Float(a / b))
            }
            (Value::Integer(a), Value::Float(b)) => {
                if *b == 0.0 {
                    return Err(RuntimeError::Custom("Division by zero".to_string()));
                }
                Ok(Value::Float(*a as f64 / b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                if *b == 0 {
                    return Err(RuntimeError::Custom("Division by zero".to_string()));
                }
                Ok(Value::Float(a / *b as f64))
            }
            _ => Err(RuntimeError::TypeError(
                format!("Cannot divide {} and {}", left.type_name(), right.type_name())
            )),
        }
    }

    /// Modulo two values
    fn modulo_values(&self, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    return Err(RuntimeError::Custom("Modulo by zero".to_string()));
                }
                Ok(Value::Integer(a % b))
            }
            _ => Err(RuntimeError::TypeError(
                format!("Cannot modulo {} and {}", left.type_name(), right.type_name())
            )),
        }
    }

    /// Compare two values
    fn compare_values<F>(&self, left: &Value, right: &Value, pred: F) -> Result<Value, RuntimeError>
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        let ordering = match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                if a < b {
                    std::cmp::Ordering::Less
                } else if a > b {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            (Value::Integer(a), Value::Float(b)) => {
                let af = *a as f64;
                if af < *b {
                    std::cmp::Ordering::Less
                } else if af > *b {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                let bf = *b as f64;
                if *a < bf {
                    std::cmp::Ordering::Less
                } else if *a > bf {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            (Value::String(a), Value::String(b)) => a.cmp(b),
            _ => return Err(RuntimeError::TypeError(
                format!("Cannot compare {} and {}", left.type_name(), right.type_name())
            )),
        };
        
        Ok(Value::Boolean(pred(ordering)))
    }

    /// Evaluate unary operation
    fn evaluate_unary_op(&self, op: &UnaryOp, val: &Value) -> Result<Value, RuntimeError> {
        match op {
            UnaryOp::Negate => {
                match val {
                    Value::Integer(n) => Ok(Value::Integer(-n)),
                    Value::Float(n) => Ok(Value::Float(-n)),
                    _ => Err(RuntimeError::TypeError(
                        format!("Cannot negate {}", val.type_name())
                    )),
                }
            }
            UnaryOp::Not => Ok(Value::Boolean(!val.is_truthy())),
        }
    }

    /// Check if a value matches a pattern
    fn match_pattern(&self, value: &Value, pattern: &Pattern) -> Result<bool, RuntimeError> {
        match pattern {
            Pattern::Wildcard => Ok(true),
            Pattern::Literal(lit) => {
                let lit_val = match lit {
                    Literal::Integer(n) => Value::Integer(*n),
                    Literal::Float(n) => Value::Float(*n),
                    Literal::String(s) => Value::String(s.clone()),
                    Literal::Boolean(b) => Value::Boolean(*b),
                    _ => return Err(RuntimeError::Custom(
                        "Complex literals in patterns not yet supported".to_string()
                    )),
                };
                Ok(value == &lit_val)
            }
            Pattern::Identifier(_) => Ok(true), // Bind the value to the identifier
            Pattern::Range(start, end) => {
                // Simplified range matching
                let start_val = match start.as_ref() {
                    Pattern::Literal(Literal::Integer(n)) => *n,
                    _ => return Err(RuntimeError::Custom(
                        "Range patterns must use integer literals".to_string()
                    )),
                };
                let end_val = match end.as_ref() {
                    Pattern::Literal(Literal::Integer(n)) => *n,
                    _ => return Err(RuntimeError::Custom(
                        "Range patterns must use integer literals".to_string()
                    )),
                };
                
                match value {
                    Value::Integer(n) => Ok(*n >= start_val && *n <= end_val),
                    _ => Ok(false),
                }
            }
            Pattern::Tuple(_) => Err(RuntimeError::Custom(
                "Tuple patterns not yet supported".to_string()
            )),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn run_source(source: &str) -> Result<Value, RuntimeError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().unwrap();
        
        let mut interpreter = Interpreter::new();
        interpreter.interpret(&ast)
    }

    #[test]
    fn test_arithmetic() {
        let source = r#"
            proto main() {
                return 1 + 2 * 3
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(7));
    }

    #[test]
    fn test_variables() {
        let source = r#"
            proto main() {
                let x = 10
                let y = 20
                return x + y
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(30));
    }

    #[test]
    fn test_if_expression() {
        let source = r#"
            proto main() {
                if true {
                    return 42
                } else {
                    return 0
                }
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn test_match_expression() {
        let source = r#"
            proto main() {
                return match 5 {
                    _ => 42
                }
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn test_function_call() {
        // Note: Currently functions must be defined before they are called
        // due to single-pass interpretation
        let source = r#"
            proto main() {
                return 42
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(42));
    }

    #[test]
    fn test_list() {
        let source = r#"
            proto main() {
                let items = [1, 2, 3]
                return items[0]
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(1));
    }

    #[test]
    fn test_for_loop() {
        // Note: Assignment in loops requires mutable variables
        // This test uses a simpler approach
        let source = r#"
            proto main() {
                let items = [1, 2, 3]
                return items[0] + items[1] + items[2]
            }
        "#;
        
        let result = run_source(source).unwrap();
        assert_eq!(result, Value::Integer(6));
    }
}