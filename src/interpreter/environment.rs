use std::collections::HashMap;
use super::value::{Value, RuntimeError};

/// Environment for variable scoping
#[derive(Debug, Clone)]
pub struct Environment {
    /// Current scope variables
    variables: HashMap<String, Value>,
    /// Parent environment (for nested scopes)
    parent: Option<Box<Environment>>,
}

impl Environment {
    /// Create a new global environment
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            parent: None,
        }
    }

    /// Create a new environment with a parent
    pub fn with_parent(parent: Environment) -> Self {
        Environment {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Define a variable in the current scope
    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Get a variable's value (searches up the scope chain)
    pub fn get(&self, name: &str) -> Result<Value, RuntimeError> {
        if let Some(value) = self.variables.get(name) {
            Ok(value.clone())
        } else if let Some(ref parent) = self.parent {
            parent.get(name)
        } else {
            Err(RuntimeError::UndefinedVariable(name.to_string()))
        }
    }

    /// Get a mutable reference to a variable (searches up the scope chain)
    pub fn get_mut(&mut self, name: &str) -> Result<&mut Value, RuntimeError> {
        if self.variables.contains_key(name) {
            self.variables.get_mut(name)
                .ok_or_else(|| RuntimeError::UndefinedVariable(name.to_string()))
        } else if let Some(ref mut parent) = self.parent {
            parent.get_mut(name)
        } else {
            Err(RuntimeError::UndefinedVariable(name.to_string()))
        }
    }

    /// Assign to an existing variable (searches up the scope chain)
    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        if self.variables.contains_key(name) {
            self.variables.insert(name.to_string(), value);
            Ok(())
        } else if let Some(ref mut parent) = self.parent {
            parent.assign(name, value)
        } else {
            Err(RuntimeError::UndefinedVariable(name.to_string()))
        }
    }

    /// Check if a variable exists in this scope or any parent scope
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name) 
            || self.parent.as_ref().map_or(false, |p| p.contains(name))
    }

    /// Get all variables in the current scope (for debugging)
    pub fn local_variables(&self) -> &HashMap<String, Value> {
        &self.variables
    }

    /// Create a snapshot of all accessible variables (for closures)
    pub fn snapshot(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        
        // First get parent variables
        if let Some(ref parent) = self.parent {
            result.extend(parent.snapshot());
        }
        
        // Then override with current scope
        result.extend(self.variables.clone());
        
        result
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}