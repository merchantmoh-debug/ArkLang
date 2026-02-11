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

use crate::ast::{ArkNode, Expression, FunctionDef, Statement};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinearError {
    #[error("Linear variable '{0}' used more than once")]
    DoubleUse(String),
    #[error("Linear variable '{0}' dropped without consumption")]
    UnusedResource(String),
    #[error("Variable '{0}' not found")]
    NotFound(String),
}

pub struct LinearChecker {
    // Tracks currently active (unconsumed) linear variables
    active_linears: HashSet<String>,
    // Tracks ALL variables declared as linear in this scope (to detect double use)
    declared_linears: HashSet<String>,
}

impl LinearChecker {
    pub fn new() -> Self {
        LinearChecker {
            active_linears: HashSet::new(),
            declared_linears: HashSet::new(),
        }
    }

    pub fn check(node: &ArkNode) -> Result<(), LinearError> {
        let mut checker = LinearChecker::new();
        checker.traverse_node(node)
    }

    pub fn check_function(&mut self, func: &FunctionDef) -> Result<(), LinearError> {
        // 1. Register input arguments
        for (name, ty) in &func.inputs {
            if ty.is_linear() {
                self.active_linears.insert(name.clone());
                self.declared_linears.insert(name.clone());
            }
        }

        self.traverse_node(&func.body.content)?;

        // 3. Verify all linear resources are consumed
        if !self.active_linears.is_empty() {
            let unused = self.active_linears.iter().next().unwrap();
            return Err(LinearError::UnusedResource(unused.clone()));
        }

        Ok(())
    }

    fn traverse_node(&mut self, node: &ArkNode) -> Result<(), LinearError> {
        match node {
            ArkNode::Statement(stmt) => self.check_statement(stmt),
            ArkNode::Expression(expr) => self.check_expression(expr),
            _ => Ok(()),
        }
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<(), LinearError> {
        match stmt {
            Statement::Let { name, ty, value } => {
                self.traverse_node(&ArkNode::Expression(value.clone()))?;

                if let Some(t) = ty {
                    if t.is_linear() {
                        self.active_linears.insert(name.clone());
                        self.declared_linears.insert(name.clone());
                    }
                }
                Ok(())
            }
            Statement::LetDestructure { names: _, value } => {
                self.traverse_node(&ArkNode::Expression(value.clone()))?;
                // Todo: Register destructuring bindings as linear if applicable
                Ok(())
            }
            Statement::SetField {
                obj_name: _,
                field: _,
                value,
            } => self.traverse_node(&ArkNode::Expression(value.clone())),
            Statement::Return(expr) => self.traverse_node(&ArkNode::Expression(expr.clone())),
            Statement::Block(stmts) => {
                for s in stmts {
                    self.check_statement(s)?;
                }
                Ok(())
            }
            Statement::Expression(expr) => self.check_expression(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_expression(condition)?;
                for stmt in then_block {
                    self.check_statement(stmt)?;
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                self.check_expression(condition)?;
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                Ok(())
            }
            Statement::Function(func_def) => {
                // Todo: Check function body with new scope?
                // For now, just traverse linear usage
                self.check_function(func_def)
            }
        }
    }

    fn check_expression(&mut self, expr: &Expression) -> Result<(), LinearError> {
        match expr {
            Expression::Variable(name) => {
                if self.active_linears.contains(name) {
                    // Consume it
                    self.active_linears.remove(name);
                    Ok(())
                } else if self.declared_linears.contains(name) {
                    // It was declared linear but is no longer active -> Double Use!
                    Err(LinearError::DoubleUse(name.clone()))
                } else {
                    // Not linear (Shared/Affine), permissible to use multiple times
                    Ok(())
                }
            }
            Expression::Call { args, .. } => {
                for arg in args {
                    self.check_expression(arg)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use crate::types::ArkType;

    #[test]
    fn test_valid_linear_usage() {
        let func = FunctionDef {
            name: "id".to_string(),
            inputs: vec![("x".to_string(), ArkType::Linear("Resource".to_string()))],
            output: ArkType::Linear("Resource".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Return(Expression::Variable(
                    "x".to_string(),
                ))))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_ok());
    }

    #[test]
    fn test_double_use_error() {
        let func = FunctionDef {
            name: "double".to_string(),
            inputs: vec![("x".to_string(), ArkType::Linear("Resource".to_string()))],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Expression(Expression::Call {
                    function_hash: "dummy".to_string(),
                    args: vec![
                        Expression::Variable("x".to_string()),
                        Expression::Variable("x".to_string()),
                    ],
                }))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        // Expect Error
        match result {
            Err(LinearError::DoubleUse(_)) => assert!(true),
            _ => panic!("Expected DoubleUse error, got {:?}", result),
        }
    }
}
