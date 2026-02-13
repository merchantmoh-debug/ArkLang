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

    fn get_intrinsic_return_linearity(name: &str) -> Vec<bool> {
        match name {
            // sys.mem.read -> [val, buf]
            "sys.mem.read" | "intrinsic_buffer_read" => vec![false, true],
            // sys.mem.write -> buf
            "sys.mem.write" | "intrinsic_buffer_write" => vec![true],
            // sys.mem.alloc -> buf
            "sys.mem.alloc" | "intrinsic_buffer_alloc" => vec![true],
            // sys.len -> [len, original_val] (original val might be linear)
            "sys.len" | "intrinsic_len" => vec![false, true],
            // sys.list.get -> [val, list]
            "sys.list.get" | "intrinsic_list_get" | "sys.str.get" => vec![false, true],
            // sys.list.append -> list
            "sys.list.append" | "intrinsic_list_append" => vec![true],
            _ => vec![false], // Default non-linear
        }
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
                // Heuristic: Check if RHS is a linear variable being moved
                // This must be done BEFORE traverse_node consumes it
                let mut inferred_linear = false;
                if let Expression::Variable(v) = value {
                    if self.declared_linears.contains(v) {
                        inferred_linear = true;
                    }
                }

                // Also check if RHS is a Call to a linear intrinsic
                if let Expression::Call { function_hash, .. } = value {
                     let sig = Self::get_intrinsic_return_linearity(function_hash);
                     if sig.len() == 1 && sig[0] {
                         inferred_linear = true;
                     }
                }

                self.traverse_node(&ArkNode::Expression(value.clone()))?;

                // Check for shadowing of active linear variable
                if self.active_linears.contains(name) {
                    return Err(LinearError::UnusedResource(name.clone()));
                }

                if inferred_linear || ty.as_ref().map(|t| t.is_linear()).unwrap_or(false) {
                    self.active_linears.insert(name.clone());
                    self.declared_linears.insert(name.clone());
                }
                Ok(())
            }
            Statement::LetDestructure { names, value } => {
                self.traverse_node(&ArkNode::Expression(value.clone()))?;

                let mut call_signature = vec![];
                if let Expression::Call { function_hash, .. } = value {
                    call_signature = Self::get_intrinsic_return_linearity(function_hash);
                }

                for (i, name) in names.iter().enumerate() {
                     let mut is_linear = false;
                     // 1. Check intrinsic signature
                     if i < call_signature.len() && call_signature[i] {
                         is_linear = true;
                     }
                     // 2. Check shadowing
                     if self.declared_linears.contains(name) {
                         is_linear = true;
                     }

                     if is_linear {
                         // Check for shadowing of active linear variable
                         if self.active_linears.contains(name) {
                             return Err(LinearError::UnusedResource(name.clone()));
                         }

                         self.active_linears.insert(name.clone());
                         self.declared_linears.insert(name.clone());
                     }
                }
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
                // Check function body with new scope to ensure isolation
                let mut function_checker = LinearChecker::new();
                function_checker.check_function(func_def)
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

    #[test]
    fn test_linear_let_unused() {
        let func = FunctionDef {
            name: "unused".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy".to_string()),
                    },
                    Statement::Return(Expression::Literal("void".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        match result {
            Err(LinearError::UnusedResource(name)) => assert_eq!(name, "x"),
            _ => panic!("Expected UnusedResource error, got {:?}", result),
        }
    }

    #[test]
    fn test_linear_let_double_use() {
        let func = FunctionDef {
            name: "double".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy".to_string()),
                    },
                    Statement::Expression(Expression::Variable("x".to_string())),
                    Statement::Expression(Expression::Variable("x".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        match result {
            Err(LinearError::DoubleUse(name)) => assert_eq!(name, "x"),
            _ => panic!("Expected DoubleUse error, got {:?}", result),
        }
    }

    #[test]
    fn test_linear_let_valid() {
        let func = FunctionDef {
            name: "valid".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy".to_string()),
                    },
                    Statement::Return(Expression::Variable("x".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_ok());
    }

    #[test]
    fn test_non_linear_let_multiple_use() {
        let func = FunctionDef {
            name: "shared".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Shared("Int".to_string())),
                        value: Expression::Literal("42".to_string()),
                    },
                    Statement::Expression(Expression::Variable("x".to_string())),
                    Statement::Expression(Expression::Variable("x".to_string())),
                    Statement::Return(Expression::Variable("x".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_ok());
    }

    #[test]
    fn test_linear_let_shadowing_leak() {
        let func = FunctionDef {
            name: "shadow_leak".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy1".to_string()),
                    },
                    // Shadowing 'x' without consuming the first one
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy2".to_string()),
                    },
                    Statement::Return(Expression::Variable("x".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        // Desired: Error. Current: Likely Ok.
        match result {
            Err(LinearError::UnusedResource(name)) => assert_eq!(name, "x"),
            Err(e) => panic!("Expected UnusedResource error, got {:?}", e),
            Ok(_) => panic!("Checker failed to catch linear variable shadowing leak!"),
        }
    }

    #[test]
    #[ignore] // TODO: Enable when type inference is implemented for linear moves
    fn test_linear_let_untyped_move() {
        let func = FunctionDef {
            name: "untyped_move".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy".to_string()),
                    },
                    Statement::Let {
                        name: "y".to_string(),
                        ty: None,
                        value: Expression::Variable("x".to_string()),
                    },
                    Statement::Expression(Expression::Variable("y".to_string())),
                    Statement::Return(Expression::Variable("y".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_err(), "Checker allowed linear resource to escape into untyped variable");
    }

    #[test]
    fn test_linear_let_shadowing_valid() {
        let func = FunctionDef {
            name: "shadow_valid".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy1".to_string()),
                    },
                    // Consume x
                    Statement::Expression(Expression::Variable("x".to_string())),
                    // Shadow x (valid because previous x was consumed)
                    Statement::Let {
                        name: "x".to_string(),
                        ty: Some(ArkType::Linear("Resource".to_string())),
                        value: Expression::Literal("dummy2".to_string()),
                    },
                    // Consume new x
                    Statement::Return(Expression::Variable("x".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Valid shadowing (after consumption) should be allowed");
    }

    #[test]
    fn test_linear_let_binding_state() {
        let stmt = Statement::Let {
            name: "x".to_string(),
            ty: Some(ArkType::Linear("Resource".to_string())),
            value: Expression::Literal("dummy".to_string()),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_statement(&stmt);

        assert!(result.is_ok());
        assert!(checker.active_linears.contains("x"));
        assert!(checker.declared_linears.contains("x"));
    }

    #[test]
    fn test_shared_let_binding_state() {
        let stmt = Statement::Let {
            name: "y".to_string(),
            ty: Some(ArkType::Shared("Int".to_string())),
            value: Expression::Literal("42".to_string()),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_statement(&stmt);

        assert!(result.is_ok());
        assert!(!checker.active_linears.contains("y"));
        assert!(!checker.declared_linears.contains("y"));
    }

    #[test]
    fn test_linear_destructure_drop_hole() {
        // let buf: Linear = ...
        // let (val, buf2) = sys.mem.read(buf, 0)
        // return val
        // buf2 is dropped but was linear!
        let func = FunctionDef {
            name: "leak_hole".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "buf".to_string(),
                        ty: Some(ArkType::Linear("Buffer".to_string())),
                        value: Expression::Literal("dummy_buf".to_string()),
                    },
                    Statement::LetDestructure {
                        names: vec!["val".to_string(), "buf2".to_string()],
                        value: Expression::Call {
                            function_hash: "sys.mem.read".to_string(),
                            args: vec![
                                Expression::Variable("buf".to_string()),
                                Expression::Literal("0".to_string())
                            ]
                        }
                    },
                    Statement::Return(Expression::Variable("val".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        match result {
            Err(LinearError::UnusedResource(name)) => assert_eq!(name, "buf2"),
            Ok(_) => panic!("Checker failed to catch linear variable leak in destructure!"),
            _ => panic!("Expected UnusedResource error, got {:?}", result),
        }
    }

    #[test]
    fn test_linear_let_call_inferred() {
        // let buf = sys.mem.alloc(10); // inferred linear
        // return buf;
        let func = FunctionDef {
            name: "alloc_inferred".to_string(),
            inputs: vec![],
            output: ArkType::Linear("Buffer".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "buf".to_string(),
                        ty: None, // No type info!
                        value: Expression::Call {
                            function_hash: "sys.mem.alloc".to_string(),
                            args: vec![Expression::Literal("10".to_string())]
                        }
                    },
                    Statement::Return(Expression::Variable("buf".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Should infer linearity from sys.mem.alloc and track it");
    }

    #[test]
    fn test_linear_let_call_inferred_leak() {
         // let buf = sys.mem.alloc(10);
         // return;
         let func = FunctionDef {
            name: "alloc_leak".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "buf".to_string(),
                        ty: None,
                        value: Expression::Call {
                            function_hash: "sys.mem.alloc".to_string(),
                            args: vec![Expression::Literal("10".to_string())]
                        }
                    },
                    Statement::Return(Expression::Literal("void".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        match result {
            Err(LinearError::UnusedResource(name)) => assert_eq!(name, "buf"),
            _ => panic!("Expected UnusedResource for leaked inferred linear var"),
        }
    }

    #[test]
    fn test_linear_destructure_shadowing_unknown() {
        // let buf: Linear = ...
        // let (val, buf) = unknown(buf) // shadowing, should infer buf is linear
        // return val // Leak buf!
         let func = FunctionDef {
            name: "shadow_unknown".to_string(),
            inputs: vec![],
            output: ArkType::Shared("Void".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Let {
                        name: "buf".to_string(),
                        ty: Some(ArkType::Linear("Buffer".to_string())),
                        value: Expression::Literal("dummy_buf".to_string()),
                    },
                    Statement::LetDestructure {
                        names: vec!["val".to_string(), "buf".to_string()],
                        value: Expression::Call {
                            function_hash: "unknown_func".to_string(),
                            args: vec![
                                Expression::Variable("buf".to_string())
                            ]
                        }
                    },
                    Statement::Return(Expression::Variable("val".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        match result {
            Err(LinearError::UnusedResource(name)) => assert_eq!(name, "buf"),
            _ => panic!("Expected UnusedResource for shadowed variable in destructure of unknown function"),
        }
    }

    #[test]
    fn test_nested_function_scope_isolation() {
        let func = FunctionDef {
            name: "outer".to_string(),
            inputs: vec![("x".to_string(), ArkType::Linear("Resource".to_string()))],
            output: ArkType::Linear("Resource".to_string()),
            body: Box::new(
                MastNode::new(ArkNode::Statement(Statement::Block(vec![
                    Statement::Function(FunctionDef {
                        name: "inner".to_string(),
                        inputs: vec![],
                        output: ArkType::Shared("Void".to_string()),
                        body: Box::new(
                            MastNode::new(ArkNode::Statement(Statement::Return(Expression::Literal(
                                "void".to_string(),
                            ))))
                            .unwrap(),
                        ),
                    }),
                    Statement::Return(Expression::Variable("x".to_string())),
                ])))
                .unwrap(),
            ),
        };

        let mut checker = LinearChecker::new();
        let result = checker.check_function(&func);
        assert!(result.is_ok(), "Nested function caused scope leak or false positive unused resource");
    }
}
