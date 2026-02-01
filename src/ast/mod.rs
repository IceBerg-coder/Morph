/// Abstract Syntax Tree definitions for Morph

use std::fmt;

/// Represents the different modes a function can be in
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionMode {
    Proto,  // Draft/interpreted mode
    Solid,  // Compiled/native mode
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    Equal,    // ==
    NotEqual, // !=
    Less,     // <
    LessEq,   // <=
    Greater,  // >
    GreaterEq,// >=
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Negate, // -
    Not,    // !
}

/// A type annotation
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Named(String),                          // e.g., "Int", "String"
    Generic(String, Vec<TypeAnnotation>),   // e.g., "List<Int>"
    Function(Vec<TypeAnnotation>, Box<TypeAnnotation>), // function type
    Ghost(Box<TypeAnnotation>, Vec<GhostAttribute>),    // Ghost type with metadata
}

/// Ghost type attributes
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
}

/// Pattern for match expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,                    // _
    Literal(Literal),           // 42, "hello", etc.
    Identifier(String),         // variable name
    Range(Box<Pattern>, Box<Pattern>), // 1..10
    Tuple(Vec<Pattern>),        // (a, b, c)
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<Expression>),
    Record(Vec<(String, Expression)>),
}

/// An expression node
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Literal value
    Literal(Literal),
    
    /// Variable reference
    Identifier(String),
    
    /// Binary operation
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    
    /// Unary operation
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    
    /// Function call
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    
    /// Pipe expression: expr |> func
    Pipe {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    
    /// Match expression
    Match {
        expr: Box<Expression>,
        arms: Vec<MatchArm>,
    },
    
    /// Block expression
    Block(Vec<Statement>),
    
    /// If expression
    If {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Option<Box<Expression>>,
    },
    
    /// Field access: obj.field
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    
    /// Index access: arr[index]
    IndexAccess {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    
    /// Lambda/closure: (params) => expr
    Lambda {
        params: Vec<Parameter>,
        body: Box<Expression>,
    },
    
    /// Claim expression: claim expr
    Claim(Box<Expression>),
}

/// A match arm: pattern => expression
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub expr: Expression,
}

/// A function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<TypeAnnotation>,
}

/// A statement
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Variable declaration: let x = expr; or var x = expr;
    VariableDecl {
        name: String,
        type_annotation: Option<TypeAnnotation>,
        initializer: Expression,
        mutable: bool, // true for var, false for let
    },
    
    /// Expression statement
    Expression(Expression),
    
    /// Return statement
    Return(Option<Expression>),
    
    /// For loop: for item in iterable { ... }
    For {
        variable: String,
        iterable: Expression,
        guard: Option<Expression>, // where clause
        body: Vec<Statement>,
    },
    
    /// Assignment: x = expr;
    Assignment {
        target: Expression,
        value: Expression,
    },
}

/// A function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub mode: FunctionMode,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Vec<Statement>,
}

/// A type declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub definition: TypeDefinition,
}

/// Type definition variants
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefinition {
    /// Type alias: type Name = OtherType
    Alias(TypeAnnotation),
    
    /// Record type: type Point = { x: Int, y: Int }
    Record(Vec<(String, TypeAnnotation)>),
    
    /// Enum type: type Color = Red | Green | Blue
    Enum(Vec<String>),
}

/// A solve block declaration
#[derive(Debug, Clone, PartialEq)]
pub struct SolveBlock {
    pub name: String,
    pub params: Vec<Parameter>,
    pub constraints: Vec<Constraint>,
    pub return_expr: Option<Expression>,
}

/// A constraint in a solve block
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Variable binding: let x = expr
    Binding {
        name: String,
        expr: Expression,
    },
    /// Ensure clause: ensure expr
    Ensure(Expression),
}

/// Import statement
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub module: String,
    pub items: Option<Vec<String>>, // None for "import module", Some for selective import
}

/// Top-level declaration in a module
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Function(FunctionDecl),
    Type(TypeDecl),
    Solve(SolveBlock),
    Import(Import),
}

/// A complete module/program
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub declarations: Vec<Declaration>,
}

impl Module {
    pub fn new() -> Self {
        Module {
            declarations: Vec::new(),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Modulo => write!(f, "%"),
            BinaryOp::Equal => write!(f, "=="),
            BinaryOp::NotEqual => write!(f, "!="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::LessEq => write!(f, "<="),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::GreaterEq => write!(f, ">="),
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}