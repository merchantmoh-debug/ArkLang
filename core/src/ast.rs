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

use hex;
use sha2::{Digest, Sha256};

/// Merkle-ized Abstract Syntax Tree Node
/// Content-Addressed by the hash of its content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MastNode {
    pub hash: String, // Hex string of SHA256 hash
    pub content: ArkNode,
}

impl MastNode {
    pub fn new(content: ArkNode) -> Self {
        let serialized = bincode::serialize(&content).unwrap(); // Todo: Handle error
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let result = hasher.finalize();
        let hash = hex::encode(result);
        MastNode { hash, content }
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
}
