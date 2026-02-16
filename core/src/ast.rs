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

/// Merkle-ized Abstract Syntax Tree Node
/// Content-Addressed by the hash of its content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MastNode {
    pub hash: String, // Hex string of SHA256 hash
    pub content: ArkNode,
}

impl MastNode {
    pub fn new(content: ArkNode) -> Result<Self, AstError> {
        let hash = calculate_hash(&content)?;
        Ok(MastNode { hash, content })
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
pub struct FunctionDef {
    pub name: String, // Human readable hint, actual ID is hash
    pub inputs: Vec<(String, ArkType)>,
    pub output: ArkType,
    pub body: Box<MastNode>, // Pointer to the body logic
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Pattern {
    Literal(String),
    Variable(String),
    Wildcard,
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
    Import(String),
    Break,
    Continue,
    Function(FunctionDef),
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
    Match {
        scrutinee: Box<Expression>,
        arms: Vec<(Pattern, Expression)>,
    },
}
