/// Token types for the Morph programming language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Keywords
    Proto,      // proto
    Solid,      // solid
    Type,       // type
    Flow,       // flow
    Let,        // let
    Var,        // var
    If,         // if
    Else,       // else
    ElseIf,     // else if
    Match,      // match
    For,        // for
    In,         // in
    Return,     // return
    Claim,      // claim
    Delegate,   // delegate
    Solve,      // solve
    Ensure,     // ensure
    Where,      // where
    Import,     // import

    // Literals
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Pipe,       // |
    PipeGreater,// |>
    Equal,      // =
    EqualEqual, // ==
    Bang,       // !
    BangEqual,  // !=
    Less,       // <
    LessEqual,  // <=
    Greater,    // >
    GreaterEqual,// >=
    Arrow,      // =>
    Dot,        // .
    DotDot,     // ..
    Colon,      // :
    ColonColon, // ::

    // Delimiters
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Comma,          // ,
    Semicolon,      // ;

    // Special
    Ghost,      // <Ghost: ...>
    Comment,    // // ...
    Newline,
    Eof,
}

/// A token with its type, literal value, and position information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, line: usize, column: usize) -> Self {
        Token {
            token_type,
            lexeme,
            line,
            column,
        }
    }

    /// Check if this token is of a specific type
    pub fn is_type(&self, token_type: &TokenType) -> bool {
        match (&self.token_type, token_type) {
            (TokenType::Identifier(_), TokenType::Identifier(_)) => true,
            (TokenType::String(_), TokenType::String(_)) => true,
            (TokenType::Integer(_), TokenType::Integer(_)) => true,
            (TokenType::Float(_), TokenType::Float(_)) => true,
            (a, b) => a == b,
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} '{}' at {}:{}", self.token_type, self.lexeme, self.line, self.column)
    }
}