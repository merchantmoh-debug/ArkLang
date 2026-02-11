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

use crate::ast::FunctionDef;

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    String(String),
    Boolean(bool),
    Unit,
    /// A linear object at runtime. Wraps internal data.
    LinearObject {
        id: String,
        typename: String,
        payload: String, // Simplified representation
    },
    Function(FunctionDef), // Closures/First-class funcs (simplified)
    List(Vec<Value>),
    Buffer(Vec<u8>),
    Struct(HashMap<String, Value>),
    /// Control Flow: Return value wrapper
    Return(Box<Value>),
}

impl Value {
    pub fn is_linear(&self) -> bool {
        match self {
            Value::Integer(_)
            | Value::Boolean(_)
            | Value::Unit
            | Value::Function(_)
            | Value::String(_) => false,
            Value::List(_)
            | Value::LinearObject { .. }
            | Value::Buffer(_)
            | Value::Struct(_) => true,
            Value::Return(val) => val.is_linear(), // Recursive check
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    variables: HashMap<String, Value>,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn new() -> Self {
        Scope {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: &'a Scope<'a>) -> Self {
        Scope {
            variables: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        match self.variables.get(name) {
            Some(v) => Some(v.clone()),
            None => match &self.parent {
                Some(p) => p.get(name),
                None => None,
            },
        }
    }

    pub fn get_or_move(&mut self, name: &str) -> Option<Value> {
        // 1. Try Local
        if let Some(v) = self.variables.get(name) {
            if v.is_linear() {
                return self.variables.remove(name);
            } else {
                return Some(v.clone());
            }
        }
        // 2. Try Parent (Only for Shared types, or implicit clone of Linear if allowed/unsafe)
        // Note: Moving out of parent is impossible with &Scope.
        // Strict Linear Type Checker prevents capturing Linear by reference if logic is sound.
        if let Some(parent) = &self.parent {
            return parent.get(name);
        }
        None
    }

    pub fn take(&mut self, name: &str) -> Option<Value> {
        if let Some(v) = self.variables.remove(name) {
            return Some(v);
        }
        // Cannot take from parent (ownership rules)?
        // For now, strict local take. If defined in parent, we can't move it out unless we have mutable ref to parent.
        // Scope struct has `parent: Option<&'a Scope>`. Immutable ref.
        // So we CANNOT move out of parent.
        // This enforces that Linear types must be passed down or local?
        // Or we need `&mut Scope` parent.
        // Changing Scope to have mutable parent... might break things.
        // For Intrinsics (Bio-Bridge) on local vars, `variables.remove` is sufficient.
        None
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
}
