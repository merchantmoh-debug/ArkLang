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
        Ok(MastNode {
            hash,
            content,
            span: None,
        })
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
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EnumVariantDef {
    pub name: String,
    pub fields: Vec<ArkType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraitDecl {
    pub name: String,
    pub methods: Vec<TraitMethodSig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraitMethodSig {
    pub name: String,
    pub inputs: Vec<(String, ArkType)>,
    pub output: ArkType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ImplBlock {
    pub trait_name: Option<String>,
    pub target_type: String,
    pub methods: Vec<FunctionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FunctionDef {
    pub name: String, // Human readable hint, actual ID is hash
    pub inputs: Vec<(String, ArkType)>,
    pub output: ArkType,
    pub body: Box<MastNode>, // Pointer to the body logic
    /// Attributes attached to this function (e.g., "export", "golem::handler")
    #[serde(default)]
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Pattern {
    Literal(String),
    Variable(String),
    Wildcard,
    EnumVariant {
        enum_name: String,
        variant: String,
        bindings: Vec<String>,
    },
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
    For {
        variable: String,
        iterable: Expression,
        body: Vec<Statement>,
    },

    Break,
    Continue,
    Function(FunctionDef),

    // New Nodes
    Import(Import),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    TraitDecl(TraitDecl),
    ImplBlock(ImplBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Expression {
    Variable(String),
    Literal(String), // Placeholder
    Integer(i64),    // Added for Number support
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
    Match {
        scrutinee: Box<Expression>,
        arms: Vec<(Pattern, Expression)>,
    },
    Lambda {
        params: Vec<String>,
        body: Vec<Statement>,
    },
    EnumInit {
        enum_name: String,
        variant: String,
        args: Vec<Expression>,
    },
}
