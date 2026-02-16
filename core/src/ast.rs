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

use crate::types::ArkType;
use serde::{Deserialize, Serialize};
use serde_json::{to_string, to_value};
use thiserror::Error;

use hex;
use sha2::{Digest, Sha256};

#[derive(Error, Debug)]
pub enum AstError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn calculate_hash<T: Serialize>(content: &T) -> Result<String, AstError> {
    // Serialize content to Canonical JSON (Sorted keys, no spaces)
    let val = to_value(content)?;
    let canonical = to_string(&val)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Span {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub file: String,
}

/// Merkle-ized Abstract Syntax Tree Node
/// Content-Addressed by the hash of its content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MastNode {
    pub hash: String, // Hex string of SHA256 hash
    pub content: ArkNode,
    pub span: Option<Span>,
}

impl MastNode {
    pub fn new(content: ArkNode) -> Result<Self, AstError> {
        let hash = calculate_hash(&content)?;
        Ok(MastNode { hash, content, span: None })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ArkNode {
    Function(FunctionDef),
    Statement(Statement),
    Expression(Expression),
    Type(ArkType),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Match {
    pub scrutinee: Box<Expression>, // Usually expressions contain boxed sub-expressions
    pub arms: Vec<(Expression, Expression)>, // Pattern -> Body
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Lambda {
    pub params: Vec<(String, ArkType)>,
    pub body: Box<MastNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TryCatch {
    pub try_block: Box<MastNode>,
    pub catch_var: String,
    pub catch_block: Box<MastNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<(String, ArkType)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FunctionDef {
    pub name: String, // Human readable hint, actual ID is hash
    pub inputs: Vec<(String, ArkType)>,
    pub output: ArkType,
    pub body: Box<MastNode>, // Pointer to the body logic
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Statement {
    Let {
        name: String,
        ty: Option<ArkType>,
        value: Expression,
    },
    LetDestructure {
        names: Vec<String>,
        value: Expression,
    },
    SetField {
        obj_name: String,
        field: String,
        value: Expression,
    },
    Return(Expression),
    Block(Vec<Statement>),
    Expression(Expression),
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Function(FunctionDef),

    // New Nodes
    Import(Import),
    StructDecl(StructDecl),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Expression {
    Variable(String),
    Literal(String), // Placeholder
    Call {
        function_hash: String,
        args: Vec<Expression>,
    },
    List(Vec<Expression>),
    StructInit {
        fields: Vec<(String, Expression)>,
    },
    GetField {
        obj: Box<Expression>,
        field: String,
    },

    // New Nodes
    Match(Match),
    Lambda(Lambda),
    TryCatch(TryCatch),
}

pub type VisitResult = ();

pub trait AstVisitor {
    fn visit_node(&mut self, node: &ArkNode) -> VisitResult {
        match node {
            ArkNode::Function(f) => self.visit_func(f),
            ArkNode::Statement(s) => self.visit_stmt(s),
            ArkNode::Expression(e) => self.visit_expr(e),
            ArkNode::Type(_) => {},
        }
    }

    fn visit_stmt(&mut self, stmt: &Statement) -> VisitResult {
        match stmt {
            Statement::Block(stmts) => self.visit_block(stmts),
            Statement::Expression(e) => self.visit_expr(e),
            Statement::If { condition, then_block, else_block } => {
                self.visit_expr(condition);
                self.visit_block(then_block);
                if let Some(else_b) = else_block {
                    self.visit_block(else_b);
                }
            }
            Statement::While { condition, body } => {
                self.visit_expr(condition);
                self.visit_block(body);
            }
            Statement::Return(e) => self.visit_expr(e),
            Statement::Let { value, .. } => self.visit_expr(value),
            Statement::LetDestructure { value, .. } => self.visit_expr(value),
            Statement::SetField { value, .. } => self.visit_expr(value),
            Statement::Function(f) => self.visit_func(f),
            Statement::Import(i) => self.visit_import(i),
            Statement::StructDecl(s) => self.visit_struct_decl(s),
        }
    }

    fn visit_expr(&mut self, expr: &Expression) -> VisitResult {
        match expr {
            Expression::Call { args, .. } => {
                for arg in args { self.visit_expr(arg); }
            },
            Expression::List(items) => {
                for item in items { self.visit_expr(item); }
            },
            Expression::StructInit { fields } => {
                for (_, e) in fields { self.visit_expr(e); }
            },
            Expression::GetField { obj, .. } => {
                self.visit_expr(obj);
            },
            Expression::Match(m) => self.visit_match(m),
            Expression::Lambda(l) => self.visit_lambda(l),
            Expression::TryCatch(t) => self.visit_try_catch(t),
            _ => {}
        }
    }

    fn visit_func(&mut self, func: &FunctionDef) -> VisitResult {
        self.visit_node(&func.body.content)
    }

    fn visit_block(&mut self, block: &[Statement]) -> VisitResult {
        for stmt in block {
            self.visit_stmt(stmt);
        }
    }

    fn visit_match(&mut self, _m: &Match) -> VisitResult {}
    fn visit_lambda(&mut self, _l: &Lambda) -> VisitResult {}
    fn visit_try_catch(&mut self, _t: &TryCatch) -> VisitResult {}
    fn visit_import(&mut self, _i: &Import) -> VisitResult {}
    fn visit_struct_decl(&mut self, _s: &StructDecl) -> VisitResult {}
}

pub fn walk_ast(visitor: &mut dyn AstVisitor, ast: &[Statement]) {
    visitor.visit_block(ast);
}

pub fn pretty_print(ast: &[Statement], indent: usize) -> String {
    let mut printer = PrettyPrinter::new(indent);
    printer.visit_block(ast);
    printer.output
}

struct PrettyPrinter {
    output: String,
    indent_level: usize,
}

impl PrettyPrinter {
    fn new(indent: usize) -> Self {
        Self { output: String::new(), indent_level: indent }
    }

    fn indent(&self) -> String {
        " ".repeat(self.indent_level)
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emit_indent(&mut self) {
        let s = self.indent();
        self.output.push_str(&s);
    }

    fn emit_line(&mut self, s: &str) {
        self.emit_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }
}

impl AstVisitor for PrettyPrinter {
    fn visit_stmt(&mut self, stmt: &Statement) -> VisitResult {
        match stmt {
            Statement::Let { name, ty, value } => {
                self.emit_indent();
                self.emit(&format!("let {}", name));
                if let Some(t) = ty {
                    self.emit(&format!(": {}", t));
                }
                self.emit(" = ");
                self.visit_expr(value);
                self.emit(";\n");
            }
            Statement::LetDestructure { names, value } => {
                self.emit_indent();
                self.emit("let (");
                for (i, name) in names.iter().enumerate() {
                    if i > 0 { self.emit(", "); }
                    self.emit(name);
                }
                self.emit(") = ");
                self.visit_expr(value);
                self.emit(";\n");
            }
            Statement::SetField { obj_name, field, value } => {
                self.emit_indent();
                self.emit(&format!("{}.{} = ", obj_name, field));
                self.visit_expr(value);
                self.emit(";\n");
            }
            Statement::Return(e) => {
                self.emit_indent();
                self.emit("return ");
                self.visit_expr(e);
                self.emit(";\n");
            }
            Statement::Block(stmts) => {
                self.emit("{\n");
                self.indent_level += 2;
                self.visit_block(stmts);
                self.indent_level -= 2;
                self.emit_indent();
                self.emit("}\n");
            }
            Statement::Expression(e) => {
                self.emit_indent();
                self.visit_expr(e);
                self.emit(";\n");
            }
            Statement::If { condition, then_block, else_block } => {
                self.emit_indent();
                self.emit("if ");
                self.visit_expr(condition);
                self.emit(" {\n");
                self.indent_level += 2;
                self.visit_block(then_block);
                self.indent_level -= 2;
                self.emit_indent();
                self.emit("}");
                if let Some(else_b) = else_block {
                    self.emit(" else {\n");
                    self.indent_level += 2;
                    self.visit_block(else_b);
                    self.indent_level -= 2;
                    self.emit_indent();
                    self.emit("}");
                }
                self.emit("\n");
            }
            Statement::While { condition, body } => {
                self.emit_indent();
                self.emit("while ");
                self.visit_expr(condition);
                self.emit(" {\n");
                self.indent_level += 2;
                self.visit_block(body);
                self.indent_level -= 2;
                self.emit_indent();
                self.emit("}\n");
            }
            Statement::Function(f) => { self.visit_func(f); }
            Statement::Import(i) => { self.visit_import(i); }
            Statement::StructDecl(s) => { self.visit_struct_decl(s); }
        }
    }

    fn visit_expr(&mut self, expr: &Expression) -> VisitResult {
        match expr {
            Expression::Variable(n) => self.emit(n),
            Expression::Literal(s) => self.emit(s),
            Expression::Call { function_hash, args } => {
                self.emit(function_hash);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit(", "); }
                    self.visit_expr(arg);
                }
                self.emit(")");
            }
            Expression::List(items) => {
                self.emit("[");
                 for (i, item) in items.iter().enumerate() {
                    if i > 0 { self.emit(", "); }
                    self.visit_expr(item);
                }
                self.emit("]");
            }
            Expression::StructInit { fields } => {
                 self.emit("Struct {");
                 for (i, (n, e)) in fields.iter().enumerate() {
                    if i > 0 { self.emit(", "); }
                    self.emit(&format!("{}: ", n));
                    self.visit_expr(e);
                }
                 self.emit("}");
            }
            Expression::GetField { obj, field } => {
                self.visit_expr(obj);
                self.emit(&format!(".{}", field));
            }
            Expression::Match(m) => { self.visit_match(m); }
            Expression::Lambda(l) => { self.visit_lambda(l); }
            Expression::TryCatch(t) => { self.visit_try_catch(t); }
        }
    }

    fn visit_func(&mut self, func: &FunctionDef) -> VisitResult {
        self.emit_indent();
        self.emit(&format!("func {}(", func.name));
        for (i, (n, t)) in func.inputs.iter().enumerate() {
             if i > 0 { self.emit(", "); }
             self.emit(&format!("{}: {}", n, t));
        }
        self.emit(&format!(") -> {} {{\n", func.output));
        self.indent_level += 2;
        self.visit_node(&func.body.content);
        self.indent_level -= 2;
        self.emit_indent();
        self.emit("}\n");
    }

    fn visit_node(&mut self, node: &ArkNode) -> VisitResult {
        match node {
             ArkNode::Statement(s) => {
                 if let Statement::Block(_) = s {
                      self.visit_stmt(s);
                 } else {
                      self.visit_stmt(s);
                 }
             },
             _ => {}
        }
    }

    fn visit_match(&mut self, m: &Match) -> VisitResult {
        self.emit_indent();
        self.emit("match ");
        self.visit_expr(&m.scrutinee);
        self.emit(" {\n");
        self.indent_level += 2;
        for (pattern, body) in &m.arms {
            self.emit_indent();
            self.visit_expr(pattern);
            self.emit(" => ");
            self.visit_expr(body);
            self.emit(",\n");
        }
        self.indent_level -= 2;
        self.emit_indent();
        self.emit("}\n");
    }

    fn visit_lambda(&mut self, l: &Lambda) -> VisitResult {
        self.emit("(");
        for (i, (n, t)) in l.params.iter().enumerate() {
             if i > 0 { self.emit(", "); }
             self.emit(&format!("{}: {}", n, t));
        }
        self.emit(") => {\n");
        self.indent_level += 2;
        self.visit_node(&l.body.content);
        self.indent_level -= 2;
        self.emit_indent();
        self.emit("}");
    }

    fn visit_try_catch(&mut self, t: &TryCatch) -> VisitResult {
        self.emit_indent();
        self.emit("try {\n");
        self.indent_level += 2;
        self.visit_node(&t.try_block.content);
        self.indent_level -= 2;
        self.emit_indent();
        self.emit(&format!("}} catch {} {{\n", t.catch_var));
        self.indent_level += 2;
        self.visit_node(&t.catch_block.content);
        self.indent_level -= 2;
        self.emit_indent();
        self.emit("}\n");
    }

    fn visit_import(&mut self, i: &Import) -> VisitResult {
        self.emit_indent();
        self.emit(&format!("import {}", i.path));
        if let Some(alias) = &i.alias {
            self.emit(&format!(" as {}", alias));
        }
        self.emit(";\n");
    }

    fn visit_struct_decl(&mut self, s: &StructDecl) -> VisitResult {
        self.emit_indent();
        self.emit(&format!("struct {} {{\n", s.name));
        self.indent_level += 2;
        for (n, t) in &s.fields {
            self.emit_indent();
            self.emit(&format!("{}: {},\n", n, t));
        }
        self.indent_level -= 2;
        self.emit_indent();
        self.emit("}\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ArkType;

    #[test]
    fn test_span_creation() {
        let span = Span {
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 10,
            file: "main.ark".to_string(),
        };
        assert_eq!(span.start_line, 1);
        assert_eq!(span.file, "main.ark");

        let stmt = Statement::Return(Expression::Literal("0".to_string()));
        let mut mast = MastNode::new(ArkNode::Statement(stmt)).unwrap();
        mast.span = Some(span.clone());
        assert_eq!(mast.span.unwrap(), span);
    }

    #[test]
    fn test_ast_pretty_print_simple() {
        let stmts = vec![
            Statement::Let {
                name: "x".to_string(),
                ty: Some(ArkType::Integer),
                value: Expression::Literal("42".to_string()),
            },
            Statement::Return(Expression::Variable("x".to_string())),
        ];

        let output = pretty_print(&stmts, 0);
        assert!(output.contains("let x: Int = 42;"));
        assert!(output.contains("return x;"));
    }

    struct FunctionCounter {
        count: usize,
    }

    impl AstVisitor for FunctionCounter {
        fn visit_func(&mut self, _func: &FunctionDef) -> VisitResult {
            self.count += 1;
        }
    }

    #[test]
    fn test_ast_visitor_counts_functions() {
        let inner_func = FunctionDef {
            name: "inner".to_string(),
            inputs: vec![],
            output: ArkType::Unit,
            body: Box::new(MastNode::new(ArkNode::Statement(Statement::Block(vec![]))).unwrap()),
        };

        let outer_func = FunctionDef {
            name: "outer".to_string(),
            inputs: vec![],
            output: ArkType::Unit,
            body: Box::new(MastNode::new(ArkNode::Function(inner_func)).unwrap()),
        };

        let stmts = vec![Statement::Function(outer_func)];

        let mut visitor = FunctionCounter { count: 0 };
        walk_ast(&mut visitor, &stmts);
        assert_eq!(visitor.count, 1);
    }

    #[test]
    fn test_match_node_construction() {
        let match_node = Match {
            scrutinee: Box::new(Expression::Variable("x".to_string())),
            arms: vec![
                (Expression::Literal("1".to_string()), Expression::Literal("one".to_string())),
            ],
        };
        // Verify we can use it in Expression
        let _expr = Expression::Match(match_node);
    }
}
