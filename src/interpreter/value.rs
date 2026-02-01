use std::collections::HashMap;
use std::fmt;
use crate::ast::{FunctionDecl, Expression};

/// Runtime values in Morph
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// List of values
    List(Vec<Value>),
    /// Record/object with fields
    Record(HashMap<String, Value>),
    /// Function value
    Function(FunctionValue),
    /// Unit/void value (for statements that don't return anything)
    Unit,
}

/// Function value that can be called
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionValue {
    /// User-defined function
    UserDefined {
        decl: FunctionDecl,
        /// Captured closure environment
        closure: Option<HashMap<String, Value>>,
    },
    /// Built-in/native function
    Builtin(BuiltinFn),
}

/// Built-in function type
pub type BuiltinFn = fn(&[Value]) -> Result<Value, RuntimeError>;

/// Runtime errors
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    TypeError(String),
    UndefinedVariable(String),
    UndefinedFunction(String),
    ArityMismatch { expected: usize, got: usize },
    IndexOutOfBounds { index: i64, len: usize },
    InvalidOperation(String),
    Custom(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::TypeError(msg) => write!(f, "Type error: {}", msg),
            RuntimeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            RuntimeError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            RuntimeError::ArityMismatch { expected, got } => {
                write!(f, "Expected {} arguments, got {}", expected, got)
            }
            RuntimeError::IndexOutOfBounds { index, len } => {
                write!(f, "Index {} out of bounds for list of length {}", index, len)
            }
            RuntimeError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            RuntimeError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl Value {
    /// Convert value to string representation
    pub fn to_string(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Float(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::List(items) => {
                let elements: Vec<String> = items.iter().map(|v| v.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            Value::Record(fields) => {
                let entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_string()))
                    .collect();
                format!("{{ {} }}", entries.join(", "))
            }
            Value::Function(_) => "<function>".to_string(),
            Value::Unit => "()".to_string(),
        }
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(items) => !items.is_empty(),
            Value::Record(fields) => !fields.is_empty(),
            Value::Function(_) => true,
            Value::Unit => false,
        }
    }

    /// Get type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "Int",
            Value::Float(_) => "Float",
            Value::String(_) => "String",
            Value::Boolean(_) => "Bool",
            Value::List(_) => "List",
            Value::Record(_) => "Record",
            Value::Function(_) => "Function",
            Value::Unit => "Unit",
        }
    }

    /// Try to convert to integer
    pub fn as_integer(&self) -> Result<i64, RuntimeError> {
        match self {
            Value::Integer(n) => Ok(*n),
            _ => Err(RuntimeError::TypeError(
                format!("Expected Int, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to float
    pub fn as_float(&self) -> Result<f64, RuntimeError> {
        match self {
            Value::Float(n) => Ok(*n),
            Value::Integer(n) => Ok(*n as f64),
            _ => Err(RuntimeError::TypeError(
                format!("Expected Float, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to string
    pub fn as_string(&self) -> Result<String, RuntimeError> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(RuntimeError::TypeError(
                format!("Expected String, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to boolean
    pub fn as_boolean(&self) -> Result<bool, RuntimeError> {
        match self {
            Value::Boolean(b) => Ok(*b),
            _ => Err(RuntimeError::TypeError(
                format!("Expected Bool, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to list
    pub fn as_list(&self) -> Result<&Vec<Value>, RuntimeError> {
        match self {
            Value::List(items) => Ok(items),
            _ => Err(RuntimeError::TypeError(
                format!("Expected List, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to mutable list
    pub fn as_list_mut(&mut self) -> Result<&mut Vec<Value>, RuntimeError> {
        match self {
            Value::List(items) => Ok(items),
            _ => Err(RuntimeError::TypeError(
                format!("Expected List, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to record
    pub fn as_record(&self) -> Result<&HashMap<String, Value>, RuntimeError> {
        match self {
            Value::Record(fields) => Ok(fields),
            _ => Err(RuntimeError::TypeError(
                format!("Expected Record, got {}", self.type_name())
            )),
        }
    }

    /// Try to convert to mutable record
    pub fn as_record_mut(&mut self) -> Result<&mut HashMap<String, Value>, RuntimeError> {
        match self {
            Value::Record(fields) => Ok(fields),
            _ => Err(RuntimeError::TypeError(
                format!("Expected Record, got {}", self.type_name())
            )),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Convert AST literal to runtime value
pub fn literal_to_value(lit: &crate::ast::Literal) -> Value {
    use crate::ast::Literal;
    
    match lit {
        Literal::Integer(n) => Value::Integer(*n),
        Literal::Float(n) => Value::Float(*n),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Boolean(b) => Value::Boolean(*b),
        Literal::List(items) => {
            let values: Vec<Value> = items.iter().map(|e| {
                // For now, we can't evaluate expressions here
                // This is handled in the interpreter
                Value::Unit
            }).collect();
            Value::List(values)
        }
        Literal::Record(fields) => {
            let mut map = HashMap::new();
            for (name, _) in fields {
                // For now, placeholder
                map.insert(name.clone(), Value::Unit);
            }
            Value::Record(map)
        }
    }
}