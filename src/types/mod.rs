use crate::ast::*;
use std::collections::HashMap;

pub mod checker;

pub use checker::{TypeChecker, validate_ghost_type};

/// Types in the Morph type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Integer type
    Int,
    /// Floating point type
    Float,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Unit/void type
    Unit,
    /// List of elements of a specific type
    List(Box<Type>),
    /// Record with named fields
    Record(HashMap<String, Type>),
    /// Function type: (param_types) -> return_type
    Function(Vec<Type>, Box<Type>),
    /// Generic type parameter
    Generic(String),
    /// Ghost type with metadata (validation in proto, stripped in solid)
    Ghost(Box<Type>, Vec<GhostAttribute>),
    /// Type variable for inference
    Variable(String),
    /// Error type for type checking failures
    Error,
}

/// Ghost type attributes for validation and optimization hints
#[derive(Debug, Clone, PartialEq)]
pub struct GhostAttribute {
    pub key: String,
    pub value: GhostValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GhostValue {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<GhostValue>),
}

/// Type errors
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    Mismatch { expected: Type, got: Type },
    UndefinedType(String),
    UndefinedVariable(String),
    ArityMismatch { expected: usize, got: usize },
    InvalidOperation(String),
    GhostValidationFailed { type_name: String, reason: String },
    Custom(String),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::Mismatch { expected, got } => {
                write!(f, "Type mismatch: expected {:?}, got {:?}", expected, got)
            }
            TypeError::UndefinedType(name) => write!(f, "Undefined type: {}", name),
            TypeError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            TypeError::ArityMismatch { expected, got } => {
                write!(f, "Expected {} arguments, got {}", expected, got)
            }
            TypeError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            TypeError::GhostValidationFailed { type_name, reason } => {
                write!(f, "Ghost type validation failed for {}: {}", type_name, reason)
            }
            TypeError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for TypeError {}

/// Type environment for tracking variable and function types
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    variables: HashMap<String, Type>,
    types: HashMap<String, Type>,
    parent: Option<Box<TypeEnvironment>>,
}

impl TypeEnvironment {
    pub fn new() -> Self {
        let mut env = TypeEnvironment {
            variables: HashMap::new(),
            types: HashMap::new(),
            parent: None,
        };
        
        // Register built-in types
        env.register_builtin_types();
        
        env
    }
    
    pub fn with_parent(parent: TypeEnvironment) -> Self {
        TypeEnvironment {
            variables: HashMap::new(),
            types: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }
    
    fn register_builtin_types(&mut self) {
        // Built-in types are implicitly defined
        self.types.insert("Int".to_string(), Type::Int);
        self.types.insert("Float".to_string(), Type::Float);
        self.types.insert("String".to_string(), Type::String);
        self.types.insert("Bool".to_string(), Type::Bool);
        self.types.insert("Unit".to_string(), Type::Unit);
    }
    
    pub fn define_variable(&mut self, name: String, ty: Type) {
        self.variables.insert(name, ty);
    }
    
    pub fn define_type(&mut self, name: String, ty: Type) {
        self.types.insert(name, ty);
    }
    
    pub fn get_variable(&self, name: &str) -> Option<Type> {
        if let Some(ty) = self.variables.get(name) {
            Some(ty.clone())
        } else if let Some(ref parent) = self.parent {
            parent.get_variable(name)
        } else {
            None
        }
    }
    
    pub fn get_type(&self, name: &str) -> Option<Type> {
        if let Some(ty) = self.types.get(name) {
            Some(ty.clone())
        } else if let Some(ref parent) = self.parent {
            parent.get_type(name)
        } else {
            None
        }
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert AST type annotation to Type
pub fn annotation_to_type(annotation: &TypeAnnotation, env: &TypeEnvironment) -> Result<Type, TypeError> {
    match annotation {
        TypeAnnotation::Named(name) => {
            env.get_type(name)
                .ok_or_else(|| TypeError::UndefinedType(name.clone()))
        }
        TypeAnnotation::Generic(name, params) => {
            // Handle generic types like List<Int>
            let param_types: Result<Vec<_>, _> = params
                .iter()
                .map(|p| annotation_to_type(p, env))
                .collect();
            
            match name.as_str() {
                "List" => {
                    let params = param_types?;
                    if params.len() != 1 {
                        return Err(TypeError::Custom(
                            "List requires exactly one type parameter".to_string()
                        ));
                    }
                    Ok(Type::List(Box::new(params[0].clone())))
                }
                _ => {
                    // For now, treat other generics as their base type
                    env.get_type(name)
                        .ok_or_else(|| TypeError::UndefinedType(name.clone()))
                }
            }
        }
        TypeAnnotation::Function(params, ret) => {
            let param_types: Result<Vec<_>, _> = params
                .iter()
                .map(|p| annotation_to_type(p, env))
                .collect();
            let ret_type = annotation_to_type(ret, env)?;
            Ok(Type::Function(param_types?, Box::new(ret_type)))
        }
        TypeAnnotation::Ghost(base, attrs) => {
            let base_type = annotation_to_type(base, env)?;
            let ghost_attrs = attrs.iter().map(|attr| GhostAttribute {
                key: attr.key.clone(),
                value: match &attr.value {
                    crate::ast::GhostValue::String(s) => GhostValue::String(s.clone()),
                    crate::ast::GhostValue::Number(n) => GhostValue::Number(*n),
                    crate::ast::GhostValue::Boolean(b) => GhostValue::Boolean(*b),
                },
            }).collect();
            Ok(Type::Ghost(Box::new(base_type), ghost_attrs))
        }
    }
}