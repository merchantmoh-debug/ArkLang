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
use std::collections::HashMap;
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

#[derive(Debug, Clone)]
struct VarState {
    is_linear: bool,
    is_active: bool,
}

pub struct LinearChecker {
    // Map variable name to stack of states (for shadowing).
    var_states: HashMap<String, Vec<VarState>>,
    // Stack of scopes. Each scope contains a list of variables declared in it.
    scope_stack: Vec<Vec<String>>,
}

impl LinearChecker {
    pub fn new() -> Self {
        LinearChecker {
            var_states: HashMap::new(),
            scope_stack: Vec::new(),
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

    // Scoping helpers
    fn enter_scope(&mut self) {
        self.scope_stack.push(Vec::new());
    }

    fn exit_scope(&mut self) -> Result<(), LinearError> {
        let scope_vars = self.scope_stack.pop().unwrap_or_default();
        // Check for unused linear resources declared in this scope (lifo order)
        for var_name in scope_vars.iter().rev() {
             if let Some(states) = self.var_states.get_mut(var_name) {
                 if let Some(state) = states.pop() {
                     if state.is_linear && state.is_active {
                         return Err(LinearError::UnusedResource(var_name.clone()));
                     }
                 }
                 // Clean up empty vector entries if needed, but not strictly required
                 if states.is_empty() {
                     self.var_states.remove(var_name);
                 }
             }
        }
        Ok(())
    }

    fn declare_var(&mut self, name: String, is_linear: bool) {
        let state = VarState {
            is_linear,
            is_active: is_linear, // Only linear vars track activity
        };
        self.var_states.entry(name.clone()).or_default().push(state);

        if let Some(current_scope) = self.scope_stack.last_mut() {
            current_scope.push(name);
        } else {
            // Fallback if not inside explicit scope (e.g. top level without enter_scope)
            // But we should always use scopes.
        }
    }

    fn use_var(&mut self, name: &str) -> Result<(), LinearError> {
        if let Some(states) = self.var_states.get_mut(name) {
            if let Some(state) = states.last_mut() {
                if state.is_linear {
                    if state.is_active {
                        state.is_active = false;
                        return Ok(());
                    } else {
                        return Err(LinearError::DoubleUse(name.to_string()));
                    }
                } else {
                    // Shared/Affine - no tracking needed
                    return Ok(());
                }
            }
        }
        // If variable not found, assume shared/global
        Ok(())
    }

    fn is_var_linear_and_active(&self, name: &str) -> bool {
        if let Some(states) = self.var_states.get(name) {
            if let Some(state) = states.last() {
                return state.is_linear && state.is_active;
            }
        }
        false
    }

    #[cfg(test)]
    pub fn is_linear_active(&self, name: &str) -> bool {
        self.is_var_linear_and_active(name)
    }

    #[cfg(test)]
    pub fn is_declared(&self, name: &str) -> bool {
        self.var_states.contains_key(name)
    }

    pub fn check_function(&mut self, func: &FunctionDef) -> Result<(), LinearError> {
        self.enter_scope();

        // 1. Register input arguments
        for (name, ty) in &func.inputs {
            let is_linear = ty.is_linear();
            self.declare_var(name.clone(), is_linear);
        }

        self.traverse_node(&func.body.content)?;

        self.exit_scope()?;

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
                let mut inferred_linear = false;

                // Peek linearity of RHS before consuming
                if let Expression::Variable(v) = value {
                    if self.is_var_linear_and_active(v) {
                        inferred_linear = true;
                    }
                }

                // Intrinsic check
                if let Expression::Call { function_hash, .. } = value {
                     let sig = Self::get_intrinsic_return_linearity(function_hash);
                     if sig.len() == 1 && sig[0] {
                         inferred_linear = true;
                     }
                }

                // Process RHS (consume linear vars)
                self.traverse_node(&ArkNode::Expression(value.clone()))?;

                // Determine linearity of new var
                let is_linear = inferred_linear || ty.as_ref().map(|t| t.is_linear()).unwrap_or(false);

                self.declare_var(name.clone(), is_linear);
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
                     // 2. Check shadowing heuristic (from original logic):
                     // If name currently maps to a linear var, assume new one is linear?
                     // This is tricky. Let's rely on type info or signature mostly.
                     // But for 'unknown_func' test compatibility, we might need to assume linearity if it was linear.
                     // But strictly speaking, shadowing shouldn't inherit type unless inferred.
                     // The original code used `declared_linears.contains(name)`.
                     if let Some(states) = self.var_states.get(name) {
                         if let Some(state) = states.last() {
                             if state.is_linear {
                                 is_linear = true;
                             }
                         }
                     }

                     self.declare_var(name.clone(), is_linear);
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
                self.enter_scope();
                for s in stmts {
                    self.check_statement(s)?;
                }
                self.exit_scope()?;
                Ok(())
            }
            Statement::Expression(expr) => self.check_expression(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_expression(condition)?;
                self.enter_scope();
                for stmt in then_block {
                    self.check_statement(stmt)?;
                }
                self.exit_scope()?;

                if let Some(else_stmts) = else_block {
                    self.enter_scope();
                    for stmt in else_stmts {
                        self.check_statement(stmt)?;
                    }
                    self.exit_scope()?;
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                self.check_expression(condition)?;
                self.enter_scope();
                for stmt in body {
                    self.check_statement(stmt)?;
                }
                self.exit_scope()?;
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
                self.use_var(name)
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
        // Manually enter scope for statement check if needed, but check_statement calls enter_scope for Block.
        // For individual Let, we must ensure declaring works.
        // declare_var handles empty stack by pushing but no scope cleanup?
        // check_statement for Let calls declare_var.
        checker.enter_scope(); // Ensure scope exists
        let result = checker.check_statement(&stmt);

        assert!(result.is_ok());
        assert!(checker.is_linear_active("x"));
        assert!(checker.is_declared("x"));
    }

    #[test]
    fn test_shared_let_binding_state() {
        let stmt = Statement::Let {
            name: "y".to_string(),
            ty: Some(ArkType::Shared("Int".to_string())),
            value: Expression::Literal("42".to_string()),
        };

        let mut checker = LinearChecker::new();
        checker.enter_scope();
        let result = checker.check_statement(&stmt);

        assert!(result.is_ok());
        assert!(!checker.is_linear_active("y"));
        // y is declared, but not linear active.
        // is_declared returns true if it exists in var_states.
        assert!(checker.is_declared("y"));
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
