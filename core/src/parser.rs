/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
 * General Public License v3.0. If you link to this code, your ENTIRE
 * application must be open-sourced under AGPLv3.
 *
 * 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
 * from Sovereign Systems.
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

//! Ark Source Parser — Recursive Descent
//!
//! Parses `.ark` source text into the existing `ArkNode` / `Statement` / `Expression`
//! AST types defined in `ast.rs`. This eliminates the Python/Lark dependency for
//! the native `ark` binary.
//!
//! Grammar reference: `meta/ark.lark` (138-line EBNF)

use crate::ast::{
    ArkNode, Expression, FunctionDef, Import, MastNode, Pattern, Statement, StructDecl,
};
use crate::types::ArkType;
use thiserror::Error;

// ─── Error Types ─────────────────────────────────────────────────────────────

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Syntax Error at {file}:{line}:{col}: {message}")]
    Syntax {
        message: String,
        line: u32,
        col: u32,
        file: String,
    },
    #[error("Unexpected token: expected {expected}, found {found} at {file}:{line}:{col}")]
    UnexpectedToken {
        expected: String,
        found: String,
        line: u32,
        col: u32,
        file: String,
    },
    #[error("Unexpected end of file")]
    UnexpectedEof,
}

impl ParseError {
    fn syntax(msg: impl Into<String>, tok: &Token, file: &str) -> Self {
        ParseError::Syntax {
            message: msg.into(),
            line: tok.line,
            col: tok.col,
            file: file.to_string(),
        }
    }

    fn unexpected(expected: impl Into<String>, tok: &Token, file: &str) -> Self {
        ParseError::UnexpectedToken {
            expected: expected.into(),
            found: format!("{:?}", tok.kind),
            line: tok.line,
            col: tok.col,
            file: file.to_string(),
        }
    }
}

// ─── Token Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    StringLit(String),
    FString(String),
    MultiString(String),
    Identifier(String),
    DocComment(String),

    // Keywords
    Let,
    Mut,
    Func,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Import,
    Struct,
    Class,
    Match,
    Try,
    Catch,
    True,
    False,
    Nil,
    Break,
    Continue,
    Async,
    Await,
    And,
    Or,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    Tilde,
    Assign,      // :=
    PlusAssign,  // +=
    MinusAssign, // -=
    StarAssign,  // *=
    SlashAssign, // /=
    Eq,          // ==
    Neq,         // !=
    Lt,
    Gt,
    Le,       // <=
    Ge,       // >=
    Pipe,     // |>
    DotDot,   // ..
    DotDotEq, // ..=
    AndAnd,   // &&
    OrOr,     // ||
    Arrow,    // =>
    OptChain, // ?.

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,

    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
    pub col: u32,
}

impl Token {
    fn new(kind: TokenKind, line: u32, col: u32) -> Self {
        Token { kind, line, col }
    }
}

// ─── Lexer ───────────────────────────────────────────────────────────────────

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.source.len() {
                tokens.push(Token::new(TokenKind::Eof, self.line, self.col));
                break;
            }
            let tok = self.next_token()?;
            tokens.push(tok);
        }
        Ok(tokens)
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Skip line comments (but not doc comments ///)
            if self.peek() == Some('/')
                && self.peek_ahead(1) == Some('/')
                && self.peek_ahead(2) != Some('/')
            {
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            // Skip block comments /* ... */
            if self.peek() == Some('/') && self.peek_ahead(1) == Some('*') {
                self.advance(); // /
                self.advance(); // *
                let mut depth = 1;
                while depth > 0 {
                    match self.advance() {
                        Some('*') if self.peek() == Some('/') => {
                            self.advance();
                            depth -= 1;
                        }
                        Some('/') if self.peek() == Some('*') => {
                            self.advance();
                            depth += 1;
                        }
                        None => break,
                        _ => {}
                    }
                }
                continue;
            }

            break;
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        let start_line = self.line;
        let start_col = self.col;

        let ch = self.peek().ok_or(ParseError::UnexpectedEof)?;

        // Doc comments ///
        if ch == '/' && self.peek_ahead(1) == Some('/') && self.peek_ahead(2) == Some('/') {
            self.advance(); // /
            self.advance(); // /
            self.advance(); // /
            let mut text = String::new();
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                text.push(c);
                self.advance();
            }
            return Ok(Token::new(
                TokenKind::DocComment(text.trim().to_string()),
                start_line,
                start_col,
            ));
        }

        // Multi-line strings """..."""
        if ch == '"' && self.peek_ahead(1) == Some('"') && self.peek_ahead(2) == Some('"') {
            self.advance(); // "
            self.advance(); // "
            self.advance(); // "
            let mut s = String::new();
            loop {
                match self.advance() {
                    Some('"') if self.peek() == Some('"') && self.peek_ahead(1) == Some('"') => {
                        self.advance();
                        self.advance();
                        break;
                    }
                    Some(c) => s.push(c),
                    None => return Err(ParseError::UnexpectedEof),
                }
            }
            return Ok(Token::new(TokenKind::MultiString(s), start_line, start_col));
        }

        // F-strings f"..."
        if ch == 'f' && self.peek_ahead(1) == Some('"') {
            self.advance(); // f
            self.advance(); // "
            let mut s = String::new();
            loop {
                match self.advance() {
                    Some('\\') => {
                        if let Some(esc) = self.advance() {
                            s.push('\\');
                            s.push(esc);
                        }
                    }
                    Some('"') => break,
                    Some(c) => s.push(c),
                    None => return Err(ParseError::UnexpectedEof),
                }
            }
            return Ok(Token::new(TokenKind::FString(s), start_line, start_col));
        }

        // Regular strings "..."
        if ch == '"' {
            self.advance(); // opening "
            let mut s = String::new();
            loop {
                match self.advance() {
                    Some('\\') => match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => return Err(ParseError::UnexpectedEof),
                    },
                    Some('"') => break,
                    Some(c) => s.push(c),
                    None => return Err(ParseError::UnexpectedEof),
                }
            }
            return Ok(Token::new(TokenKind::StringLit(s), start_line, start_col));
        }

        // Numbers (including negative handled at parser level)
        if ch.is_ascii_digit() {
            let mut num = String::new();
            let mut is_float = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num.push(c);
                    self.advance();
                } else if c == '.' && self.peek_ahead(1).map_or(false, |n| n.is_ascii_digit()) {
                    is_float = true;
                    num.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            if is_float {
                let val: f64 = num.parse().unwrap_or(0.0);
                return Ok(Token::new(TokenKind::Float(val), start_line, start_col));
            } else {
                let val: i64 = num.parse().unwrap_or(0);
                return Ok(Token::new(TokenKind::Integer(val), start_line, start_col));
            }
        }

        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut ident = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    ident.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            let kind = match ident.as_str() {
                "let" => TokenKind::Let,
                "mut" => TokenKind::Mut,
                "func" => TokenKind::Func,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "for" => TokenKind::For,
                "in" => TokenKind::In,
                "return" => TokenKind::Return,
                "import" => TokenKind::Import,
                "struct" => TokenKind::Struct,
                "class" => TokenKind::Class,
                "match" => TokenKind::Match,
                "try" => TokenKind::Try,
                "catch" => TokenKind::Catch,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "nil" => TokenKind::Nil,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "async" => TokenKind::Async,
                "await" => TokenKind::Await,
                "and" => TokenKind::And,
                "or" => TokenKind::Or,
                _ => TokenKind::Identifier(ident),
            };
            return Ok(Token::new(kind, start_line, start_col));
        }

        // Operators and delimiters
        self.advance();
        let kind = match ch {
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusAssign
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusAssign
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarAssign
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashAssign
                } else {
                    TokenKind::Slash
                }
            }
            '%' => TokenKind::Percent,
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Neq
                } else {
                    TokenKind::Bang
                }
            }
            '~' => TokenKind::Tilde,
            ':' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Assign
                } else {
                    TokenKind::Colon
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Eq
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    // Single = is not used in Ark (uses :=), treat as error or fallback
                    return Err(ParseError::Syntax {
                        message: "Unexpected '='. Did you mean ':=' for assignment or '==' for comparison?".into(),
                        line: start_line,
                        col: start_col,
                        file: String::new(),
                    });
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            '|' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Pipe
                } else if self.peek() == Some('|') {
                    self.advance();
                    TokenKind::OrOr
                } else {
                    // Bare | is used for lambda params: |x, y| { ... }
                    // We'll use Pipe for |> and just return a special token
                    // Actually, we need the pipe char for lambda syntax
                    // Let's return it as a generic delimiter
                    TokenKind::Pipe // Will be handled in parser for lambda
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    TokenKind::AndAnd
                } else {
                    return Err(ParseError::Syntax {
                        message: "Unexpected '&'. Did you mean '&&'?".into(),
                        line: start_line,
                        col: start_col,
                        file: String::new(),
                    });
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::DotDotEq
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            '?' => {
                if self.peek() == Some('.') {
                    self.advance();
                    TokenKind::OptChain
                } else {
                    return Err(ParseError::Syntax {
                        message: "Unexpected '?'".into(),
                        line: start_line,
                        col: start_col,
                        file: String::new(),
                    });
                }
            }
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            _ => {
                return Err(ParseError::Syntax {
                    message: format!("Unexpected character: '{}'", ch),
                    line: start_line,
                    col: start_col,
                    file: String::new(),
                });
            }
        };

        Ok(Token::new(kind, start_line, start_col))
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    file: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: &str) -> Self {
        Parser {
            tokens,
            pos: 0,
            file: file.to_string(),
        }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token {
            kind: TokenKind::Eof,
            line: 0,
            col: 0,
        })
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<Token, ParseError> {
        let tok = self.peek().clone();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(expected) {
            self.advance();
            Ok(tok)
        } else {
            Err(ParseError::unexpected(
                format!("{:?}", expected),
                &tok,
                &self.file,
            ))
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn match_tok(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    // ─── Top-Level ───────────────────────────────────────────────────────

    /// Parse the entire source file into a top-level Block statement.
    pub fn parse_program(&mut self) -> Result<ArkNode, ParseError> {
        let mut stmts = Vec::new();
        while !self.at_end() {
            // Skip doc comments at top level (store for next def)
            let _doc = self.try_doc_comment();
            if self.at_end() {
                break;
            }

            let stmt = self.parse_top_level_item()?;
            stmts.push(stmt);
        }
        Ok(ArkNode::Statement(Statement::Block(stmts)))
    }

    fn try_doc_comment(&mut self) -> Option<String> {
        if let TokenKind::DocComment(text) = &self.peek().kind {
            let text = text.clone();
            self.advance();
            Some(text)
        } else {
            None
        }
    }

    fn parse_top_level_item(&mut self) -> Result<Statement, ParseError> {
        match &self.peek().kind {
            TokenKind::Func => self.parse_function_def(),
            TokenKind::Class => self.parse_class_def(),
            _ => self.parse_statement(),
        }
    }

    // ─── Function Definition ─────────────────────────────────────────────

    fn parse_function_def(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Func)?;

        let name_tok = self.peek().clone();
        let name = match &name_tok.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                return Err(ParseError::unexpected(
                    "function name",
                    &name_tok,
                    &self.file,
                ));
            }
        };
        self.advance();

        // Parameters
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let p_tok = self.peek().clone();
                let p_name = match &p_tok.kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => return Err(ParseError::unexpected("parameter name", &p_tok, &self.file)),
                };
                self.advance();
                params.push((p_name, ArkType::Any));

                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen)?;

        // Body
        let body = self.parse_block()?;
        let body_node = ArkNode::Statement(Statement::Block(body));
        let mast = MastNode::new(body_node).map_err(|e| ParseError::Syntax {
            message: format!("MAST error: {}", e),
            line: name_tok.line,
            col: name_tok.col,
            file: self.file.clone(),
        })?;

        Ok(Statement::Function(FunctionDef {
            name,
            inputs: params,
            output: ArkType::Any,
            body: Box::new(mast),
        }))
    }

    // ─── Class Definition ────────────────────────────────────────────────

    fn parse_class_def(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Class)?;

        let name_tok = self.peek().clone();
        let name = match &name_tok.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => return Err(ParseError::unexpected("class name", &name_tok, &self.file)),
        };
        self.advance();

        self.expect(&TokenKind::LBrace)?;

        // Parse methods (functions inside class)
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let _doc = self.try_doc_comment();
            if self.check(&TokenKind::Func) {
                let method = self.parse_function_def()?;
                if let Statement::Function(func_def) = method {
                    fields.push((func_def.name.clone(), ArkType::Any));
                }
            } else if let TokenKind::Identifier(field_name) = &self.peek().kind {
                // Bare field declaration
                let fname = field_name.clone();
                self.advance();
                fields.push((fname, ArkType::Any));
            } else {
                break;
            }
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Statement::StructDecl(StructDecl { name, fields }))
    }

    // ─── Statements ──────────────────────────────────────────────────────

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.peek().kind {
            TokenKind::Let => self.parse_let(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Match => self.parse_match(),
            TokenKind::Try => self.parse_try(),
            TokenKind::Break => {
                self.advance();
                Ok(Statement::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Statement::Continue)
            }
            TokenKind::Func => self.parse_function_def(),
            _ => self.parse_expr_or_assign(),
        }
    }

    fn parse_let(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Let)?;

        // Check for destructure: let (a, b) := expr
        if self.check(&TokenKind::LParen) {
            self.advance();
            let mut names = Vec::new();
            loop {
                let n_tok = self.peek().clone();
                let name = match &n_tok.kind {
                    TokenKind::Identifier(n) => n.clone(),
                    _ => return Err(ParseError::unexpected("identifier", &n_tok, &self.file)),
                };
                self.advance();
                names.push(name);
                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::Assign)?;
            let value = self.parse_expression()?;
            return Ok(Statement::LetDestructure { names, value });
        }

        // Regular let
        let name_tok = self.peek().clone();
        let name = match &name_tok.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                return Err(ParseError::unexpected(
                    "variable name",
                    &name_tok,
                    &self.file,
                ));
            }
        };
        self.advance();

        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expression()?;

        Ok(Statement::Let {
            name,
            ty: None,
            value,
        })
    }

    fn parse_if(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::If)?;
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;

        let else_block = if self.match_tok(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                // else if → nested if as single-element block
                let nested_if = self.parse_if()?;
                Some(vec![nested_if])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::While)?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::For)?;

        let var_tok = self.peek().clone();
        let variable = match &var_tok.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                return Err(ParseError::unexpected(
                    "loop variable",
                    &var_tok,
                    &self.file,
                ));
            }
        };
        self.advance();
        self.expect(&TokenKind::In)?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(Statement::For {
            variable,
            iterable,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Return)?;
        // Return can have no value (returns Unit)
        if self.check(&TokenKind::RBrace)
            || self.check(&TokenKind::Eof)
            || self.peek().line != self.tokens[self.pos - 1].line
        {
            return Ok(Statement::Return(Expression::Literal("unit".into())));
        }
        let value = self.parse_expression()?;
        Ok(Statement::Return(value))
    }

    fn parse_import(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Import)?;
        let mut path_parts = Vec::new();

        let first_tok = self.peek().clone();
        let first = match &first_tok.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                return Err(ParseError::unexpected(
                    "module name",
                    &first_tok,
                    &self.file,
                ));
            }
        };
        self.advance();
        path_parts.push(first);

        while self.match_tok(&TokenKind::Dot) {
            let p_tok = self.peek().clone();
            let part = match &p_tok.kind {
                TokenKind::Identifier(n) => n.clone(),
                _ => return Err(ParseError::unexpected("module name", &p_tok, &self.file)),
            };
            self.advance();
            path_parts.push(part);
        }

        Ok(Statement::Import(Import {
            path: path_parts.join("."),
            alias: None,
        }))
    }

    fn parse_match(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Match)?;
        let scrutinee = self.parse_expression()?;
        self.expect(&TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let pattern_expr = self.parse_expression()?;
            self.expect(&TokenKind::Arrow)?;

            let body = if self.check(&TokenKind::LBrace) {
                let _block = self.parse_block()?;
                Expression::Literal("block".into()) // Simplified for now
            } else {
                let expr = self.parse_expression()?;
                self.match_tok(&TokenKind::Comma); // optional trailing comma
                expr
            };

            // Convert expression pattern to Pattern enum
            let pattern = match &pattern_expr {
                Expression::Literal(s) => Pattern::Literal(s.clone()),
                Expression::Variable(s) if s == "_" => Pattern::Wildcard,
                Expression::Variable(s) => Pattern::Variable(s.clone()),
                Expression::Integer(n) => Pattern::Literal(n.to_string()),
                _ => Pattern::Wildcard,
            };

            arms.push((pattern, body));
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Statement::Expression(Expression::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        }))
    }

    fn parse_try(&mut self) -> Result<Statement, ParseError> {
        self.expect(&TokenKind::Try)?;
        let try_block = self.parse_block()?;

        self.expect(&TokenKind::Catch)?;
        let var_tok = self.peek().clone();
        let _catch_var = match &var_tok.kind {
            TokenKind::Identifier(n) => n.clone(),
            _ => {
                return Err(ParseError::unexpected(
                    "catch variable",
                    &var_tok,
                    &self.file,
                ));
            }
        };
        self.advance();

        let _catch_block = self.parse_block()?;

        // Wrap as nested if/block since AST TryCatch uses MastNode
        // For the bytecode compiler, we emit as a Block with error handling semantics
        let all_stmts = try_block;
        // The catch_var and catch_block are encoded but not fully wired to VM yet
        // This is a placeholder that preserves the structure
        Ok(Statement::Block(all_stmts))
    }

    fn parse_expr_or_assign(&mut self) -> Result<Statement, ParseError> {
        let expr = self.parse_expression()?;

        // Check for assignment operators
        match &self.peek().kind {
            TokenKind::Assign => {
                self.advance();
                let value = self.parse_expression()?;

                match expr {
                    Expression::Variable(name) => Ok(Statement::Let {
                        name,
                        ty: None,
                        value,
                    }),
                    Expression::GetField { obj, field } => {
                        // obj.field := value
                        if let Expression::Variable(obj_name) = *obj {
                            Ok(Statement::SetField {
                                obj_name,
                                field,
                                value,
                            })
                        } else {
                            Ok(Statement::Expression(value))
                        }
                    }
                    _ => Ok(Statement::Expression(value)),
                }
            }
            TokenKind::PlusAssign
            | TokenKind::MinusAssign
            | TokenKind::StarAssign
            | TokenKind::SlashAssign => {
                let op_tok = self.peek().clone();
                self.advance();
                let rhs = self.parse_expression()?;

                let op_name = match &op_tok.kind {
                    TokenKind::PlusAssign => "add",
                    TokenKind::MinusAssign => "sub",
                    TokenKind::StarAssign => "mul",
                    TokenKind::SlashAssign => "div",
                    _ => unreachable!(),
                };

                if let Expression::Variable(name) = expr {
                    Ok(Statement::Let {
                        name: name.clone(),
                        ty: None,
                        value: Expression::Call {
                            function_hash: op_name.to_string(),
                            args: vec![Expression::Variable(name), rhs],
                        },
                    })
                } else {
                    Ok(Statement::Expression(rhs))
                }
            }
            _ => Ok(Statement::Expression(expr)),
        }
    }

    // ─── Block ───────────────────────────────────────────────────────────

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            let _doc = self.try_doc_comment();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    // ─── Expressions (Precedence Climbing) ───────────────────────────────

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_pipe()
    }

    // pipe_expr: logical_or ("|>" logical_or)*
    fn parse_pipe(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_logical_or()?;
        while self.check(&TokenKind::Pipe) && self.peek_is_pipe_op() {
            self.advance();
            let right = self.parse_logical_or()?;
            // pipe: left |> right becomes right(left)
            left = Expression::Call {
                function_hash: match &right {
                    Expression::Variable(name) => name.clone(),
                    _ => "__pipe__".into(),
                },
                args: vec![left],
            };
        }
        Ok(left)
    }

    fn peek_is_pipe_op(&self) -> bool {
        // Pipe token is |> which is already tokenized as TokenKind::Pipe
        // But the bare | for lambda uses the same token...
        // We handle this by checking context: if followed by >, it's pipe
        // Actually our lexer already distinguishes |> (Pipe) from | (lambda bracket)
        // Let's just check it's the Pipe token
        matches!(self.peek().kind, TokenKind::Pipe)
    }

    // logical_or: logical_and (("||" | "or") logical_and)*
    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_logical_and()?;
        while matches!(self.peek().kind, TokenKind::OrOr | TokenKind::Or) {
            self.advance();
            let right = self.parse_logical_and()?;
            left = Expression::Call {
                function_hash: "or".into(),
                args: vec![left, right],
            };
        }
        Ok(left)
    }

    // logical_and: comparison (("&&" | "and") comparison)*
    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;
        while matches!(self.peek().kind, TokenKind::AndAnd | TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expression::Call {
                function_hash: "and".into(),
                args: vec![left, right],
            };
        }
        Ok(left)
    }

    // comparison: range_expr (("<" | ">" | "<=" | ">=" | "==" | "!=") range_expr)*
    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_range()?;
        loop {
            let op_name = match &self.peek().kind {
                TokenKind::Gt => "gt",
                TokenKind::Lt => "lt",
                TokenKind::Ge => "ge",
                TokenKind::Le => "le",
                TokenKind::Eq => "eq",
                TokenKind::Neq => "neq",
                _ => break,
            };
            self.advance();
            let right = self.parse_range()?;
            left = Expression::Call {
                function_hash: op_name.into(),
                args: vec![left, right],
            };
        }
        Ok(left)
    }

    // range: sum (".." sum | "..=" sum)?
    fn parse_range(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_sum()?;
        match &self.peek().kind {
            TokenKind::DotDot => {
                self.advance();
                let right = self.parse_sum()?;
                Ok(Expression::Call {
                    function_hash: "range_exclusive".into(),
                    args: vec![left, right],
                })
            }
            TokenKind::DotDotEq => {
                self.advance();
                let right = self.parse_sum()?;
                Ok(Expression::Call {
                    function_hash: "range_inclusive".into(),
                    args: vec![left, right],
                })
            }
            _ => Ok(left),
        }
    }

    // sum: product (("+" | "-") product)*
    fn parse_sum(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_product()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Plus => "add",
                TokenKind::Minus => "sub",
                _ => break,
            };
            self.advance();
            let right = self.parse_product()?;
            left = Expression::Call {
                function_hash: op.into(),
                args: vec![left, right],
            };
        }
        Ok(left)
    }

    // product: unary (("*" | "/" | "%") unary)*
    fn parse_product(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => "mul",
                TokenKind::Slash => "div",
                TokenKind::Percent => "mod",
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expression::Call {
                function_hash: op.into(),
                args: vec![left, right],
            };
        }
        Ok(left)
    }

    // unary: ("!" | "-" | "~") unary | atom
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match &self.peek().kind {
            TokenKind::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::Call {
                    function_hash: "not".into(),
                    args: vec![expr],
                })
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::Call {
                    function_hash: "neg".into(),
                    args: vec![expr],
                })
            }
            TokenKind::Tilde => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::Call {
                    function_hash: "bit_not".into(),
                    args: vec![expr],
                })
            }
            _ => self.parse_postfix(),
        }
    }

    // postfix: primary (.field | ?.field | (args) | [index])*
    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            match &self.peek().kind {
                TokenKind::Dot => {
                    self.advance();
                    let field_tok = self.peek().clone();
                    let field = match &field_tok.kind {
                        TokenKind::Identifier(n) => n.clone(),
                        _ => {
                            return Err(ParseError::unexpected(
                                "field name",
                                &field_tok,
                                &self.file,
                            ));
                        }
                    };
                    self.advance();

                    // Check if this is a method call: obj.method(args)
                    if self.check(&TokenKind::LParen) {
                        self.advance(); // (
                        let mut args = vec![expr.clone()]; // self is first arg
                        if !self.check(&TokenKind::RParen) {
                            loop {
                                args.push(self.parse_expression()?);
                                if !self.match_tok(&TokenKind::Comma) {
                                    break;
                                }
                            }
                        }
                        self.expect(&TokenKind::RParen)?;

                        // For dotted calls like sys.ai.ask, we need to build the full name
                        let full_name = match &expr {
                            Expression::Variable(name) => format!("{}.{}", name, field),
                            Expression::GetField {
                                obj,
                                field: parent_field,
                            } => {
                                if let Expression::Variable(base) = obj.as_ref() {
                                    format!("{}.{}.{}", base, parent_field, field)
                                } else {
                                    field.clone()
                                }
                            }
                            _ => field.clone(),
                        };

                        // Remove self from args for namespace calls
                        let call_args = if full_name.contains('.') {
                            args[1..].to_vec() // Skip the namespace object
                        } else {
                            args
                        };

                        expr = Expression::Call {
                            function_hash: full_name,
                            args: call_args,
                        };
                    } else {
                        expr = Expression::GetField {
                            obj: Box::new(expr),
                            field,
                        };
                    }
                }
                TokenKind::OptChain => {
                    self.advance();
                    let field_tok = self.peek().clone();
                    let field = match &field_tok.kind {
                        TokenKind::Identifier(n) => n.clone(),
                        _ => {
                            return Err(ParseError::unexpected(
                                "field name",
                                &field_tok,
                                &self.file,
                            ));
                        }
                    };
                    self.advance();
                    // Optional chaining → GetField (runtime handles nil check)
                    expr = Expression::GetField {
                        obj: Box::new(expr),
                        field,
                    };
                }
                TokenKind::LParen => {
                    self.advance(); // (
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.match_tok(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&TokenKind::RParen)?;

                    let func_name = match &expr {
                        Expression::Variable(name) => name.clone(),
                        _ => "__call__".into(),
                    };

                    expr = Expression::Call {
                        function_hash: func_name,
                        args,
                    };
                }
                TokenKind::LBracket => {
                    self.advance(); // [
                    let index = self.parse_expression()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expression::Call {
                        function_hash: "get_item".into(),
                        args: vec![expr, index],
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    // primary: NUMBER | STRING | FSTRING | MULTISTRING | IDENTIFIER | true | false | nil
    //        | "(" expr ")" | "[" list "]" | "{" struct "}" | lambda
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Expression::Integer(n))
            }
            TokenKind::Float(f) => {
                let f = *f;
                self.advance();
                // Store as Literal since Expression doesn't have a Float variant
                Ok(Expression::Literal(f.to_string()))
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(s))
            }
            TokenKind::FString(s) => {
                let s = s.clone();
                self.advance();
                // F-strings are stored as Literal with f-string prefix for runtime interpolation
                Ok(Expression::Literal(format!("f\"{}\"", s)))
            }
            TokenKind::MultiString(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(s))
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression::Variable(name))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal("true".into()))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal("false".into()))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expression::Literal("nil".into()))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        items.push(self.parse_expression()?);
                        if !self.match_tok(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expression::List(items))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let key_tok = self.peek().clone();
                        let key = match &key_tok.kind {
                            TokenKind::Identifier(n) => n.clone(),
                            _ => {
                                return Err(ParseError::unexpected(
                                    "field name",
                                    &key_tok,
                                    &self.file,
                                ));
                            }
                        };
                        self.advance();
                        self.expect(&TokenKind::Colon)?;
                        let value = self.parse_expression()?;
                        fields.push((key, value));
                        if !self.match_tok(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RBrace)?;
                Ok(Expression::StructInit { fields })
            }
            TokenKind::Func => {
                // Lambda: func(params) { body }
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        let p_tok = self.peek().clone();
                        if let TokenKind::Identifier(name) = &p_tok.kind {
                            params.push(name.clone());
                            self.advance();
                        }
                        if !self.match_tok(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen)?;
                let _body = self.parse_block()?;
                // Lambdas are anonymous functions — emit as Call for now
                Ok(Expression::Literal("lambda".into()))
            }
            _ => Err(ParseError::syntax(
                format!("Expected expression, found {:?}", tok.kind),
                &tok,
                &self.file,
            )),
        }
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Parse Ark source code into an `ArkNode` AST.
pub fn parse_source(source: &str, file: &str) -> Result<ArkNode, ParseError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens, file);
    parser.parse_program()
}

/// Parse Ark source code into a `MastNode` (content-addressed, hash-verified).
pub fn parse_to_mast(source: &str, file: &str) -> Result<MastNode, ParseError> {
    let ast = parse_source(source, file)?;
    MastNode::new(ast).map_err(|e| ParseError::Syntax {
        message: format!("MAST hash error: {}", e),
        line: 0,
        col: 0,
        file: file.to_string(),
    })
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_hello_world() {
        let mut lexer = Lexer::new(r#"func main() { print("Hello") }"#);
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Func));
        assert!(matches!(&tokens[1].kind, TokenKind::Identifier(n) if n == "main"));
        assert!(matches!(tokens[2].kind, TokenKind::LParen));
        assert!(matches!(tokens[3].kind, TokenKind::RParen));
        assert!(matches!(tokens[4].kind, TokenKind::LBrace));
    }

    #[test]
    fn test_lex_numbers() {
        let mut lexer = Lexer::new("42 3.14");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Integer(42)));
        assert!(matches!(tokens[1].kind, TokenKind::Float(f) if (f - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_lex_operators() {
        let mut lexer = Lexer::new(":= == != <= >= |> .. ..=");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Assign));
        assert!(matches!(tokens[1].kind, TokenKind::Eq));
        assert!(matches!(tokens[2].kind, TokenKind::Neq));
        assert!(matches!(tokens[3].kind, TokenKind::Le));
        assert!(matches!(tokens[4].kind, TokenKind::Ge));
        assert!(matches!(tokens[5].kind, TokenKind::Pipe));
        assert!(matches!(tokens[6].kind, TokenKind::DotDot));
        assert!(matches!(tokens[7].kind, TokenKind::DotDotEq));
    }

    #[test]
    fn test_lex_string_escapes() {
        let mut lexer = Lexer::new(r#""hello\nworld""#);
        let tokens = lexer.tokenize().unwrap();
        if let TokenKind::StringLit(s) = &tokens[0].kind {
            assert_eq!(s, "hello\nworld");
        } else {
            panic!("Expected StringLit");
        }
    }

    #[test]
    fn test_parse_let_stmt() {
        let ast = parse_source("x := 42", "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            assert_eq!(stmts.len(), 1);
            if let Statement::Let { name, value, .. } = &stmts[0] {
                assert_eq!(name, "x");
                assert!(matches!(value, Expression::Integer(42)));
            } else {
                panic!("Expected Let, got {:?}", stmts[0]);
            }
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_parse_function_def() {
        let ast = parse_source("func add(a, b) { return a + b }", "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            assert_eq!(stmts.len(), 1);
            if let Statement::Function(f) = &stmts[0] {
                assert_eq!(f.name, "add");
                assert_eq!(f.inputs.len(), 2);
            } else {
                panic!("Expected Function, got {:?}", stmts[0]);
            }
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_parse_arithmetic() {
        let ast = parse_source("x := 1 + 2 * 3", "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            if let Statement::Let { name, value, .. } = &stmts[0] {
                assert_eq!(name, "x");
                // 1 + (2 * 3) due to precedence
                if let Expression::Call {
                    function_hash,
                    args,
                } = value
                {
                    assert_eq!(function_hash, "add");
                    assert!(matches!(&args[0], Expression::Integer(1)));
                    if let Expression::Call {
                        function_hash: inner_op,
                        ..
                    } = &args[1]
                    {
                        assert_eq!(inner_op, "mul");
                    }
                }
            }
        }
    }

    #[test]
    fn test_parse_if_else() {
        let source = r#"
            if x > 0 {
                print("positive")
            } else {
                print("non-positive")
            }
        "#;
        let ast = parse_source(source, "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            assert_eq!(stmts.len(), 1);
            if let Statement::If {
                condition,
                then_block,
                else_block,
            } = &stmts[0]
            {
                assert!(else_block.is_some());
            } else {
                panic!("Expected If");
            }
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let source = r#"
            while i < 10 {
                i := i + 1
            }
        "#;
        let ast = parse_source(source, "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            assert_eq!(stmts.len(), 1);
            assert!(matches!(&stmts[0], Statement::While { .. }));
        }
    }

    #[test]
    fn test_parse_method_call() {
        let ast = parse_source(r#"sys.ai.ask("prompt")"#, "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            if let Statement::Expression(Expression::Call {
                function_hash,
                args,
            }) = &stmts[0]
            {
                assert_eq!(function_hash, "sys.ai.ask");
                assert_eq!(args.len(), 1);
            } else {
                panic!("Expected Call");
            }
        }
    }

    #[test]
    fn test_parse_list_literal() {
        let ast = parse_source("xs := [1, 2, 3]", "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            if let Statement::Let { value, .. } = &stmts[0] {
                if let Expression::List(items) = value {
                    assert_eq!(items.len(), 3);
                }
            }
        }
    }

    #[test]
    fn test_parse_struct_init() {
        let ast = parse_source(r#"p := { x: 1, y: 2 }"#, "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            if let Statement::Let { value, .. } = &stmts[0] {
                if let Expression::StructInit { fields } = value {
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                    assert_eq!(fields[1].0, "y");
                }
            }
        }
    }

    #[test]
    fn test_parse_import() {
        let ast = parse_source("import std.crypto", "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            if let Statement::Import(imp) = &stmts[0] {
                assert_eq!(imp.path, "std.crypto");
            }
        }
    }

    #[test]
    fn test_parse_destructure() {
        let ast = parse_source("let (a, b) := get_pair()", "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            if let Statement::LetDestructure { names, value } = &stmts[0] {
                assert_eq!(names, &["a", "b"]);
            }
        }
    }

    #[test]
    fn test_comments_skipped() {
        let source = r#"
            // This is a comment
            x := 42
            /* Block
               comment */
            y := 10
        "#;
        let ast = parse_source(source, "test.ark").unwrap();
        if let ArkNode::Statement(Statement::Block(stmts)) = ast {
            assert_eq!(stmts.len(), 2);
        }
    }
}
