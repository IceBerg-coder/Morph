use super::token::{Token, TokenType};
use anyhow::{Result, bail};

/// Lexer for the Morph programming language
pub struct Lexer {
    /// Source code being lexed
    source: String,
    /// Current position in source
    current: usize,
    /// Start position of current token
    start: usize,
    /// Current line number
    line: usize,
    /// Current column number
    column: usize,
}

impl Lexer {
    /// Create a new lexer from source code
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.to_string(),
            current: 0,
            start: 0,
            line: 1,
            column: 1,
        }
    }

    /// Tokenize the entire source and return all tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.start = self.current;
            let token = self.next_token()?;
            tokens.push(token);
        }

        // Add EOF token
        tokens.push(Token::new(
            TokenType::Eof,
            "".to_string(),
            self.line,
            self.column,
        ));

        Ok(tokens)
    }

    /// Get the next token from the source
    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return Ok(self.make_token(TokenType::Eof));
        }

        let c = self.advance();

        match c {
            // Single-character tokens
            '(' => Ok(self.make_token(TokenType::LeftParen)),
            ')' => Ok(self.make_token(TokenType::RightParen)),
            '{' => Ok(self.make_token(TokenType::LeftBrace)),
            '}' => Ok(self.make_token(TokenType::RightBrace)),
            '[' => Ok(self.make_token(TokenType::LeftBracket)),
            ']' => Ok(self.make_token(TokenType::RightBracket)),
            ',' => Ok(self.make_token(TokenType::Comma)),
            ';' => Ok(self.make_token(TokenType::Semicolon)),
            '+' => Ok(self.make_token(TokenType::Plus)),
            '-' => Ok(self.make_token(TokenType::Minus)),
            '*' => Ok(self.make_token(TokenType::Star)),
            '/' => {
                if self.match_char('/') {
                    // Comment - consume until newline
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                    Ok(self.make_token(TokenType::Comment))
                } else {
                    Ok(self.make_token(TokenType::Slash))
                }
            }
            '%' => Ok(self.make_token(TokenType::Percent)),
            '!' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenType::BangEqual))
                } else {
                    Ok(self.make_token(TokenType::Bang))
                }
            }
            '=' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenType::EqualEqual))
                } else if self.match_char('>') {
                    Ok(self.make_token(TokenType::Arrow))
                } else {
                    Ok(self.make_token(TokenType::Equal))
                }
            }
            '<' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenType::LessEqual))
                } else if self.match_char('-') {
                    // Handle <- assignment (if needed)
                    Ok(self.make_token(TokenType::Less))
                } else {
                    Ok(self.make_token(TokenType::Less))
                }
            }
            '>' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenType::GreaterEqual))
                } else {
                    Ok(self.make_token(TokenType::Greater))
                }
            }
            '|' => {
                if self.match_char('>') {
                    Ok(self.make_token(TokenType::PipeGreater))
                } else {
                    Ok(self.make_token(TokenType::Pipe))
                }
            }
            ':' => {
                if self.match_char(':') {
                    Ok(self.make_token(TokenType::ColonColon))
                } else {
                    Ok(self.make_token(TokenType::Colon))
                }
            }
            '.' => {
                if self.match_char('.') {
                    Ok(self.make_token(TokenType::DotDot))
                } else {
                    Ok(self.make_token(TokenType::Dot))
                }
            }
            '\n' => {
                self.line += 1;
                self.column = 1;
                Ok(self.make_token(TokenType::Newline))
            }
            '"' => self.string(),
            c if c.is_ascii_digit() => self.number(),
            c if c.is_ascii_alphabetic() || c == '_' => self.identifier(),
            _ => bail!("Unexpected character '{}' at line {}, column {}", c, self.line, self.column),
        }
    }

    /// Parse a string literal
    fn string(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_column = self.column;

        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            bail!("Unterminated string at line {}, column {}", start_line, start_column);
        }

        // Consume closing quote
        self.advance();

        let value = self.source[self.start + 1..self.current - 1].to_string();
        Ok(Token::new(
            TokenType::String(value),
            self.source[self.start..self.current].to_string(),
            start_line,
            start_column,
        ))
    }

    /// Parse a number (integer or float)
    fn number(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_column = self.column;

        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // Check for decimal point
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // Consume '.'
            while self.peek().is_ascii_digit() {
                self.advance();
            }

            let value: f64 = self.source[self.start..self.current].parse()?;
            Ok(Token::new(
                TokenType::Float(value),
                self.source[self.start..self.current].to_string(),
                start_line,
                start_column,
            ))
        } else {
            let value: i64 = self.source[self.start..self.current].parse()?;
            Ok(Token::new(
                TokenType::Integer(value),
                self.source[self.start..self.current].to_string(),
                start_line,
                start_column,
            ))
        }
    }

    /// Parse an identifier or keyword
    fn identifier(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_column = self.column;

        while self.peek().is_ascii_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        let token_type = self.keyword_or_identifier(text);

        Ok(Token::new(
            token_type,
            text.to_string(),
            start_line,
            start_column,
        ))
    }

    /// Check if text is a keyword or identifier
    fn keyword_or_identifier(&self, text: &str) -> TokenType {
        match text {
            "proto" => TokenType::Proto,
            "solid" => TokenType::Solid,
            "type" => TokenType::Type,
            "flow" => TokenType::Flow,
            "let" => TokenType::Let,
            "var" => TokenType::Var,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "match" => TokenType::Match,
            "for" => TokenType::For,
            "in" => TokenType::In,
            "return" => TokenType::Return,
            "claim" => TokenType::Claim,
            "delegate" => TokenType::Delegate,
            "solve" => TokenType::Solve,
            "ensure" => TokenType::Ensure,
            "where" => TokenType::Where,
            "import" => TokenType::Import,
            "true" => TokenType::Boolean(true),
            "false" => TokenType::Boolean(false),
            _ => TokenType::Identifier(text.to_string()),
        }
    }

    /// Skip whitespace characters (except newlines)
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Check if we've reached the end of source
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    /// Get the current character and advance
    fn advance(&mut self) -> char {
        let c = self.source.chars().nth(self.current).unwrap_or('\0');
        self.current += 1;
        self.column += 1;
        c
    }

    /// Peek at the current character without advancing
    fn peek(&self) -> char {
        self.source.chars().nth(self.current).unwrap_or('\0')
    }

    /// Peek at the next character
    fn peek_next(&self) -> char {
        self.source.chars().nth(self.current + 1).unwrap_or('\0')
    }

    /// Match and consume a specific character
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source.chars().nth(self.current) != Some(expected) {
            return false;
        }
        self.current += 1;
        self.column += 1;
        true
    }

    /// Create a token from the current position
    fn make_token(&self, token_type: TokenType) -> Token {
        Token::new(
            token_type,
            self.source[self.start..self.current].to_string(),
            self.line,
            self.column - (self.current - self.start),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let source = "proto solid let var if else match for in return";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].token_type, TokenType::Proto);
        assert_eq!(tokens[1].token_type, TokenType::Solid);
        assert_eq!(tokens[2].token_type, TokenType::Let);
        assert_eq!(tokens[3].token_type, TokenType::Var);
        assert_eq!(tokens[4].token_type, TokenType::If);
        assert_eq!(tokens[5].token_type, TokenType::Else);
        assert_eq!(tokens[6].token_type, TokenType::Match);
        assert_eq!(tokens[7].token_type, TokenType::For);
        assert_eq!(tokens[8].token_type, TokenType::In);
        assert_eq!(tokens[9].token_type, TokenType::Return);
    }

    #[test]
    fn test_operators() {
        let source = "+ - * / % | |> = == ! != < <= > >= => .. ::";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].token_type, TokenType::Plus);
        assert_eq!(tokens[1].token_type, TokenType::Minus);
        assert_eq!(tokens[2].token_type, TokenType::Star);
        assert_eq!(tokens[3].token_type, TokenType::Slash);
        assert_eq!(tokens[4].token_type, TokenType::Percent);
        assert_eq!(tokens[5].token_type, TokenType::Pipe);
        assert_eq!(tokens[6].token_type, TokenType::PipeGreater);
        assert_eq!(tokens[7].token_type, TokenType::Equal);
        assert_eq!(tokens[8].token_type, TokenType::EqualEqual);
        assert_eq!(tokens[9].token_type, TokenType::Bang);
        assert_eq!(tokens[10].token_type, TokenType::BangEqual);
        assert_eq!(tokens[11].token_type, TokenType::Less);
        assert_eq!(tokens[12].token_type, TokenType::LessEqual);
        assert_eq!(tokens[13].token_type, TokenType::Greater);
        assert_eq!(tokens[14].token_type, TokenType::GreaterEqual);
        assert_eq!(tokens[15].token_type, TokenType::Arrow);
        assert_eq!(tokens[16].token_type, TokenType::DotDot);
        assert_eq!(tokens[17].token_type, TokenType::ColonColon);
    }

    #[test]
    fn test_string() {
        let source = r#""hello world""#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        match &tokens[0].token_type {
            TokenType::String(s) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string token"),
        }
    }

    #[test]
    fn test_numbers() {
        let source = "42 3.14";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        match &tokens[0].token_type {
            TokenType::Integer(n) => assert_eq!(*n, 42),
            _ => panic!("Expected integer token"),
        }

        match &tokens[1].token_type {
            TokenType::Float(n) => assert_eq!(*n, 3.14),
            _ => panic!("Expected float token"),
        }
    }

    #[test]
    fn test_pipe_example() {
        let source = "url |> fetch |> parse |> process |> log";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();

        assert!(matches!(tokens[0].token_type, TokenType::Identifier(_)));
        assert_eq!(tokens[1].token_type, TokenType::PipeGreater);
        assert!(matches!(tokens[2].token_type, TokenType::Identifier(_)));
        assert_eq!(tokens[3].token_type, TokenType::PipeGreater);
    }
}