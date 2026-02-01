use crate::ast::*;
use crate::lexer::{Token, TokenType};
use anyhow::{Result, bail};

/// Parser for Morph language
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    /// Parse the tokens into a Module (AST)
    pub fn parse(&mut self) -> Result<Module> {
        let mut module = Module::new();

        while !self.is_at_end() {
            // Skip newlines between declarations
            self.skip_newlines();
            
            if self.is_at_end() {
                break;
            }

            let decl = self.parse_declaration()?;
            module.declarations.push(decl);
        }

        Ok(module)
    }

    /// Parse a top-level declaration
    fn parse_declaration(&mut self) -> Result<Declaration> {
        self.skip_newlines();

        match self.peek().token_type {
            TokenType::Proto | TokenType::Solid => {
                let func = self.parse_function()?;
                Ok(Declaration::Function(func))
            }
            TokenType::Type => {
                let type_decl = self.parse_type_declaration()?;
                Ok(Declaration::Type(type_decl))
            }
            TokenType::Solve => {
                let solve = self.parse_solve_block()?;
                Ok(Declaration::Solve(solve))
            }
            TokenType::Import => {
                let import = self.parse_import()?;
                Ok(Declaration::Import(import))
            }
            _ => bail!(
                "Unexpected token '{}' at line {}, column {}. Expected declaration.",
                self.peek().lexeme,
                self.peek().line,
                self.peek().column
            ),
        }
    }

    /// Parse a function declaration
    fn parse_function(&mut self) -> Result<FunctionDecl> {
        // Parse mode (proto or solid)
        let mode = if self.match_token(TokenType::Proto) {
            FunctionMode::Proto
        } else if self.match_token(TokenType::Solid) {
            FunctionMode::Solid
        } else {
            bail!("Expected 'proto' or 'solid' at line {}", self.peek().line);
        };

        // Parse function name
        let name = self.consume_identifier("function name")?;

        // Parse parameters
        self.consume(TokenType::LeftParen, "'(' after function name")?;
        let params = self.parse_parameters()?;
        self.consume(TokenType::RightParen, "')' after parameters")?;

        // Parse return type (optional)
        let return_type = if self.match_token(TokenType::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        // Parse body
        self.consume(TokenType::LeftBrace, "'{' before function body")?;
        let body = self.parse_block()?;

        Ok(FunctionDecl {
            mode,
            name,
            params,
            return_type,
            body,
        })
    }

    /// Parse function parameters
    fn parse_parameters(&mut self) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();

        if self.check(TokenType::RightParen) {
            return Ok(params);
        }

        loop {
            let name = self.consume_identifier("parameter name")?;
            
            let type_annotation = if self.match_token(TokenType::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            params.push(Parameter {
                name,
                type_annotation,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(params)
    }

    /// Parse a type annotation
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation> {
        let name = self.consume_identifier("type name")?;

        // Check for generic type
        if self.match_token(TokenType::Less) {
            let mut params = Vec::new();
            
            loop {
                params.push(self.parse_type_annotation()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            
            self.consume(TokenType::Greater, "'>' after generic parameters")?;
            
            // Check for Ghost type attributes
            if self.match_token(TokenType::Less) {
                if let TokenType::Identifier(ref s) = self.peek().token_type {
                    if s == "Ghost" {
                        // Parse Ghost attributes
                        self.advance(); // consume Ghost
                        self.consume(TokenType::Colon, "':' after Ghost")?;
                        
                        let mut attributes = Vec::new();
                        // Parse Ghost attributes (simplified)
                        while !self.check(TokenType::Greater) && !self.is_at_end() {
                            self.advance();
                        }
                        self.consume(TokenType::Greater, "'>' after Ghost attributes")?;
                        
                        return Ok(TypeAnnotation::Ghost(
                            Box::new(TypeAnnotation::Generic(name, params)),
                            attributes,
                        ));
                    }
                }
            }
            
            Ok(TypeAnnotation::Generic(name, params))
        } else {
            Ok(TypeAnnotation::Named(name))
        }
    }

    /// Parse a block of statements
    fn parse_block(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();

        self.skip_newlines();

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        self.consume(TokenType::RightBrace, "'}' after block")?;
        Ok(statements)
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Statement> {
        self.skip_newlines();

        match self.peek().token_type {
            TokenType::Let | TokenType::Var => self.parse_variable_decl(),
            TokenType::Return => self.parse_return(),
            TokenType::For => self.parse_for_loop(),
            _ => {
                // Try to parse as expression statement
                let expr = self.parse_expression()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    /// Parse variable declaration (let or var)
    fn parse_variable_decl(&mut self) -> Result<Statement> {
        let mutable = self.match_token(TokenType::Var);
        if !mutable {
            self.consume(TokenType::Let, "'let' or 'var'")?;
        }

        let name = self.consume_identifier("variable name")?;

        let type_annotation = if self.match_token(TokenType::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        self.consume(TokenType::Equal, "'=' after variable name")?;
        let initializer = self.parse_expression()?;

        Ok(Statement::VariableDecl {
            name,
            type_annotation,
            initializer,
            mutable,
        })
    }

    /// Parse return statement
    fn parse_return(&mut self) -> Result<Statement> {
        self.consume(TokenType::Return, "'return'")?;

        let value = if self.check(TokenType::Newline) 
            || self.check(TokenType::RightBrace) 
            || self.check(TokenType::Eof) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        Ok(Statement::Return(value))
    }

    /// Parse for loop
    fn parse_for_loop(&mut self) -> Result<Statement> {
        self.consume(TokenType::For, "'for'")?;
        let variable = self.consume_identifier("loop variable")?;
        self.consume(TokenType::In, "'in' after loop variable")?;
        let iterable = self.parse_expression()?;

        // Parse optional where clause
        let guard = if self.match_token(TokenType::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "'{' before loop body")?;
        let body = self.parse_block()?;

        Ok(Statement::For {
            variable,
            iterable,
            guard,
            body,
        })
    }

    /// Parse type declaration
    fn parse_type_declaration(&mut self) -> Result<TypeDecl> {
        self.consume(TokenType::Type, "'type'")?;
        let name = self.consume_identifier("type name")?;
        self.consume(TokenType::Equal, "'=' after type name")?;

        let definition = if self.match_token(TokenType::LeftBrace) {
            // Record type
            let mut fields = Vec::new();
            
            loop {
                self.skip_newlines();
                if self.check(TokenType::RightBrace) {
                    break;
                }
                
                let field_name = self.consume_identifier("field name")?;
                self.consume(TokenType::Colon, "':' after field name")?;
                let field_type = self.parse_type_annotation()?;
                fields.push((field_name, field_type));
                
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            
            self.consume(TokenType::RightBrace, "'}' after record fields")?;
            TypeDefinition::Record(fields)
        } else {
            // Type alias
            TypeDefinition::Alias(self.parse_type_annotation()?)
        };

        Ok(TypeDecl { name, definition })
    }

    /// Parse solve block
    fn parse_solve_block(&mut self) -> Result<SolveBlock> {
        self.consume(TokenType::Solve, "'solve'")?;
        let name = self.consume_identifier("solve block name")?;
        
        self.consume(TokenType::LeftParen, "'(' after solve name")?;
        let params = self.parse_parameters()?;
        self.consume(TokenType::RightParen, "')' after solve parameters")?;
        
        self.consume(TokenType::LeftBrace, "'{' before solve body")?;
        
        let mut constraints = Vec::new();
        let mut return_expr = None;
        
        self.skip_newlines();
        
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            if self.match_token(TokenType::Let) {
                let name = self.consume_identifier("variable name")?;
                self.consume(TokenType::Equal, "'=' after variable name")?;
                let expr = self.parse_expression()?;
                constraints.push(Constraint::Binding { name, expr });
            } else if self.match_token(TokenType::Ensure) {
                let expr = self.parse_expression()?;
                constraints.push(Constraint::Ensure(expr));
            } else if self.match_token(TokenType::Return) {
                return_expr = Some(self.parse_expression()?);
            } else {
                bail!("Unexpected token in solve block at line {}", self.peek().line);
            }
            
            self.skip_newlines();
        }
        
        self.consume(TokenType::RightBrace, "'}' after solve block")?;
        
        Ok(SolveBlock {
            name,
            params,
            constraints,
            return_expr,
        })
    }

    /// Parse import statement
    fn parse_import(&mut self) -> Result<Import> {
        self.consume(TokenType::Import, "'import'")?;
        let module = self.consume_identifier("module name")?;
        
        // TODO: Handle selective imports
        let items = None;
        
        Ok(Import { module, items })
    }

    /// Parse expression (handles pipe operator)
    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_pipe()
    }

    /// Parse pipe expressions (lowest precedence)
    fn parse_pipe(&mut self) -> Result<Expression> {
        let mut expr = self.parse_or()?;

        while self.match_token(TokenType::PipeGreater) {
            let right = self.parse_or()?;
            expr = Expression::Pipe {
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse logical OR (not in Morph spec but for completeness)
    fn parse_or(&mut self) -> Result<Expression> {
        self.parse_and()
    }

    /// Parse logical AND
    fn parse_and(&mut self) -> Result<Expression> {
        self.parse_equality()
    }

    /// Parse equality operators
    fn parse_equality(&mut self) -> Result<Expression> {
        let mut expr = self.parse_comparison()?;

        while self.match_tokens(&[TokenType::EqualEqual, TokenType::BangEqual]) {
            let op = if self.previous().token_type == TokenType::EqualEqual {
                BinaryOp::Equal
            } else {
                BinaryOp::NotEqual
            };
            let right = self.parse_comparison()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse comparison operators
    fn parse_comparison(&mut self) -> Result<Expression> {
        let mut expr = self.parse_term()?;

        while self.match_tokens(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let op = match self.previous().token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEq,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEq,
                _ => unreachable!(),
            };
            let right = self.parse_term()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse addition and subtraction
    fn parse_term(&mut self) -> Result<Expression> {
        let mut expr = self.parse_factor()?;

        while self.match_tokens(&[TokenType::Minus, TokenType::Plus]) {
            let op = if self.previous().token_type == TokenType::Plus {
                BinaryOp::Add
            } else {
                BinaryOp::Subtract
            };
            let right = self.parse_factor()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse multiplication, division, modulo
    fn parse_factor(&mut self) -> Result<Expression> {
        let mut expr = self.parse_unary()?;

        while self.match_tokens(&[TokenType::Slash, TokenType::Star, TokenType::Percent]) {
            let op = match self.previous().token_type {
                TokenType::Slash => BinaryOp::Divide,
                TokenType::Star => BinaryOp::Multiply,
                TokenType::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    /// Parse unary operators
    fn parse_unary(&mut self) -> Result<Expression> {
        if self.match_tokens(&[TokenType::Bang, TokenType::Minus]) {
            let op = if self.previous().token_type == TokenType::Bang {
                UnaryOp::Not
            } else {
                UnaryOp::Negate
            };
            let expr = self.parse_unary()?;
            return Ok(Expression::Unary {
                op,
                expr: Box::new(expr),
            });
        }

        self.parse_call()
    }

    /// Parse function calls
    fn parse_call(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(TokenType::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(TokenType::Dot) {
                let field = self.consume_identifier("field name")?;
                expr = Expression::FieldAccess {
                    object: Box::new(expr),
                    field,
                };
            } else if self.match_token(TokenType::LeftBracket) {
                let index = self.parse_expression()?;
                self.consume(TokenType::RightBracket, "']' after index")?;
                expr = Expression::IndexAccess {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Finish parsing a function call
    fn finish_call(&mut self, callee: Expression) -> Result<Expression> {
        let mut args = Vec::new();

        if !self.check(TokenType::RightParen) {
            loop {
                args.push(self.parse_expression()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "')' after arguments")?;

        Ok(Expression::Call {
            callee: Box::new(callee),
            args,
        })
    }

    /// Parse primary expressions
    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek().token_type {
            TokenType::Boolean(b) => {
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(b)))
            }
            TokenType::Integer(n) => {
                self.advance();
                Ok(Expression::Literal(Literal::Integer(n)))
            }
            TokenType::Float(n) => {
                self.advance();
                Ok(Expression::Literal(Literal::Float(n)))
            }
            TokenType::String(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            TokenType::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression::Identifier(name))
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(TokenType::RightParen, "')' after expression")?;
                Ok(expr)
            }
            TokenType::LeftBrace => {
                self.advance();
                // Check if this is a record literal or a block
                if self.check(TokenType::RightBrace) {
                    // Empty record literal
                    self.advance();
                    Ok(Expression::Literal(Literal::Record(vec![])))
                } else if self.is_record_literal() {
                    self.parse_record_literal()
                } else {
                    let statements = self.parse_block()?;
                    Ok(Expression::Block(statements))
                }
            }
            TokenType::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                
                if !self.check(TokenType::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                }
                
                self.consume(TokenType::RightBracket, "']' after list elements")?;
                Ok(Expression::Literal(Literal::List(elements)))
            }
            TokenType::If => self.parse_if_expression(),
            TokenType::Match => self.parse_match_expression(),
            TokenType::Claim => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Expression::Claim(Box::new(expr)))
            }
            _ => bail!(
                "Unexpected token '{}' at line {}, column {}",
                self.peek().lexeme,
                self.peek().line,
                self.peek().column
            ),
        }
    }

    /// Parse if expression
    fn parse_if_expression(&mut self) -> Result<Expression> {
        self.consume(TokenType::If, "'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenType::LeftBrace, "'{' after if condition")?;
        let then_branch = Box::new(Expression::Block(self.parse_block()?));

        let else_branch = if self.match_token(TokenType::Else) {
            if self.check(TokenType::If) {
                // else if
                Some(Box::new(self.parse_if_expression()?))
            } else {
                self.consume(TokenType::LeftBrace, "'{' after else")?;
                Some(Box::new(Expression::Block(self.parse_block()?)))
            }
        } else {
            None
        };

        Ok(Expression::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
        })
    }

    /// Parse match expression
    fn parse_match_expression(&mut self) -> Result<Expression> {
        self.consume(TokenType::Match, "'match'")?;
        let expr = self.parse_expression()?;
        self.consume(TokenType::LeftBrace, "'{' after match expression")?;

        let mut arms = Vec::new();
        
        self.skip_newlines();
        
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern()?;
            self.consume(TokenType::Arrow, "'=>' after pattern")?;
            let arm_expr = self.parse_expression()?;
            
            // Optional comma
            self.match_token(TokenType::Comma);
            
            arms.push(MatchArm {
                pattern,
                expr: arm_expr,
            });
            
            self.skip_newlines();
        }

        self.consume(TokenType::RightBrace, "'}' after match arms")?;

        Ok(Expression::Match {
            expr: Box::new(expr),
            arms,
        })
    }

    /// Parse a pattern
    fn parse_pattern(&mut self) -> Result<Pattern> {
        match self.peek().token_type {
            TokenType::Identifier(ref s) if s == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenType::Integer(n) => {
                self.advance();
                
                // Check for range pattern (e.g., 90..100)
                if self.match_token(TokenType::DotDot) {
                    let end = self.parse_pattern()?;
                    return Ok(Pattern::Range(
                        Box::new(Pattern::Literal(Literal::Integer(n))),
                        Box::new(end),
                    ));
                }
                
                Ok(Pattern::Literal(Literal::Integer(n)))
            }
            TokenType::Float(n) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Float(n)))
            }
            TokenType::String(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            TokenType::Boolean(b) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Boolean(b)))
            }
            TokenType::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Pattern::Identifier(name))
            }
            _ => bail!("Unexpected token in pattern at line {}", self.peek().line),
        }
    }

    /// Check if the current position looks like a record literal
    /// A record literal starts with { identifier: ... }
    fn is_record_literal(&self) -> bool {
        // We need to look ahead: if we see { identifier : ... } it's a record
        // If we see { identifier (not :) it's a block
        let mut idx = self.current;
        
        // Check if we're at an identifier
        if let TokenType::Identifier(_) = &self.tokens[idx].token_type {
            idx += 1;
            // Check if next token is a colon
            if idx < self.tokens.len() {
                return matches!(self.tokens[idx].token_type, TokenType::Colon);
            }
        }
        false
    }

    /// Parse a record literal: { field1: expr1, field2: expr2, ... }
    fn parse_record_literal(&mut self) -> Result<Expression> {
        let mut fields = Vec::new();
        
        loop {
            // Parse field name (identifier)
            let field_name = self.consume_identifier("field name")?;
            
            // Consume the colon
            self.consume(TokenType::Colon, "':' after field name")?;
            
            // Parse the field value expression
            let value = self.parse_expression()?;
            
            fields.push((field_name, value));
            
            // Check for comma or end of record
            if !self.match_token(TokenType::Comma) {
                break;
            }
            
            // Allow trailing comma by checking for closing brace
            if self.check(TokenType::RightBrace) {
                break;
            }
        }
        
        self.consume(TokenType::RightBrace, "'}' after record fields")?;
        Ok(Expression::Literal(Literal::Record(fields)))
    }

    // Helper methods

    fn skip_newlines(&mut self) {
        while self.match_token(TokenType::Newline) || self.match_token(TokenType::Comment) {
            // Skip
        }
    }

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_tokens(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if self.check(t.clone()) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().is_type(&token_type)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<()> {
        if self.check(token_type) {
            self.advance();
            Ok(())
        } else {
            bail!(
                "Expected {} at line {}, column {}. Got '{}' instead.",
                message,
                self.peek().line,
                self.peek().column,
                self.peek().lexeme
            )
        }
    }

    fn consume_identifier(&mut self, description: &str) -> Result<String> {
        match &self.peek().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => bail!(
                "Expected {} at line {}, column {}. Got '{}' instead.",
                description,
                self.peek().line,
                self.peek().column,
                self.peek().lexeme
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_source(source: &str) -> Result<Module> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_simple_function() {
        let source = r#"
            proto add(a, b) {
                return a + b
            }
        "#;
        
        let module = parse_source(source).unwrap();
        assert_eq!(module.declarations.len(), 1);
        
        match &module.declarations[0] {
            Declaration::Function(func) => {
                assert_eq!(func.name, "add");
                assert_eq!(func.params.len(), 2);
                assert!(matches!(func.mode, FunctionMode::Proto));
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_pipe_expression() {
        let source = r#"
            proto process(url) {
                url |> fetch |> parse |> log
            }
        "#;
        
        let module = parse_source(source).unwrap();
        assert_eq!(module.declarations.len(), 1);
    }

    #[test]
    fn test_match_expression() {
        let source = r#"
            proto grade(score) {
                return match score {
                    90..100 => "A",
                    80..89 => "B",
                    _ => "C"
                }
            }
        "#;
        
        let module = parse_source(source).unwrap();
        assert_eq!(module.declarations.len(), 1);
    }

    #[test]
    fn test_variable_declarations() {
        let source = r#"
            proto test() {
                let x = 42
                var y = "hello"
                return x
            }
        "#;
        
        let module = parse_source(source).unwrap();
        assert_eq!(module.declarations.len(), 1);
    }
}
