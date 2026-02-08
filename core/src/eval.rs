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

use crate::ast::{ArkNode, Expression, Statement};
use crate::runtime::{Scope, Value};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvalError {
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error("Type mismatch: expected {0}, got {1:?}")]
    TypeMismatch(String, Value),
    #[error("Not executable")]
    NotExecutable,
}

pub struct Interpreter;

impl Interpreter {
    pub fn eval(node: &ArkNode, scope: &mut Scope) -> Result<Value, EvalError> {
        match node {
            ArkNode::Statement(stmt) => Interpreter::eval_statement(stmt, scope),
            ArkNode::Expression(expr) => Interpreter::eval_expression(expr, scope),
            ArkNode::Function(func_def) => {
                scope.set(func_def.name.clone(), Value::Function(func_def.clone()));
                Ok(Value::Unit)
            }
            _ => Ok(Value::Unit),
        }
    }

    fn eval_statement(stmt: &Statement, scope: &mut Scope) -> Result<Value, EvalError> {
        match stmt {
            Statement::Let { name, ty: _, value } => {
                let val = Interpreter::eval_expression(value, scope)?;
                scope.set(name.clone(), val);
                Ok(Value::Unit)
            }
            Statement::Return(expr) => Interpreter::eval_expression(expr, scope),
            Statement::Block(stmts) => {
                let mut last_val = Value::Unit;
                for stmt in stmts {
                    last_val = Interpreter::eval_statement(stmt, scope)?;
                    // Todo: Handle early return check if we had return values in statements
                }
                Ok(last_val)
            }
            Statement::Expression(expr) => Interpreter::eval_expression(expr, scope),
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = Interpreter::eval_expression(condition, scope)?;
                let is_true = match cond_val {
                    Value::Integer(i) => i != 0,
                    Value::Boolean(b) => b,
                    _ => false,
                };

                let mut result = Value::Unit;
                if is_true {
                    for stmt in then_block {
                        result = Interpreter::eval_statement(stmt, scope)?;
                    }
                } else if let Some(stmts) = else_block {
                    for stmt in stmts {
                        result = Interpreter::eval_statement(stmt, scope)?;
                    }
                }
                Ok(result)
            }
            Statement::While { condition, body } => {
                loop {
                    let cond_val = Interpreter::eval_expression(condition, scope)?;
                    let is_true = match cond_val {
                        Value::Integer(i) => i != 0,
                        Value::Boolean(b) => b,
                        _ => false,
                    };

                    if !is_true {
                        break;
                    }

                    for stmt in body {
                        Interpreter::eval_statement(stmt, scope)?;
                    }
                }
                Ok(Value::Unit)
            }
            Statement::Function(func_def) => {
                scope.set(func_def.name.clone(), Value::Function(func_def.clone()));
                Ok(Value::Unit)
            }
        }
    }

    fn eval_expression(expr: &Expression, scope: &mut Scope) -> Result<Value, EvalError> {
        match expr {
            Expression::Literal(s) => {
                if let Ok(i) = s.parse::<i64>() {
                    Ok(Value::Integer(i))
                } else if s == "true" {
                    Ok(Value::Boolean(true))
                } else if s == "false" {
                    Ok(Value::Boolean(false))
                } else {
                    Ok(Value::String(s.clone()))
                }
            }
            Expression::Variable(name) => scope
                .get(name)
                .ok_or_else(|| EvalError::VariableNotFound(name.clone())),
            Expression::Call {
                function_hash,
                args,
            } => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(Interpreter::eval_expression(arg, scope)?);
                }

                if let Some(native_fn) =
                    crate::intrinsics::IntrinsicRegistry::resolve(function_hash)
                {
                    return native_fn(evaluated_args);
                }

                // User Function Lookup
                if let Some(val) = scope.get(function_hash) {
                    if let Value::Function(func_def) = val {
                        // Create new scope with parent
                        let mut call_scope = Scope::with_parent(scope);

                        // Bind args
                        if evaluated_args.len() != func_def.inputs.len() {
                            println!(
                                "Arity mismatch: expected {}, got {}",
                                func_def.inputs.len(),
                                evaluated_args.len()
                            );
                            return Err(EvalError::NotExecutable); // Arity mismatch
                        }

                        for (i, (param_name, _)) in func_def.inputs.iter().enumerate() {
                            call_scope.set(param_name.clone(), evaluated_args[i].clone());
                        }

                        // Eval body
                        // We need to recursively evaluate the AST node.
                        // Since func_def.body is MastNode, we need to extract content.
                        // Currently MastNode holds ArkNode.
                        // Wait, func_def.body is Box<MastNode>.
                        return Interpreter::eval(&func_def.body.content, &mut call_scope);
                    } else {
                        println!(
                            "Found variable '{}' but it is not a function: {:?}",
                            function_hash, val
                        );
                    }
                } else {
                    println!("Function '{}' not found in scope.", function_hash);
                }

                Err(EvalError::NotExecutable)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expression;

    #[test]
    fn test_eval_arithmetic() {
        let mut scope = Scope::new();

        // 5 + 3
        let expr = Expression::Call {
            function_hash: "intrinsic_add".to_string(),
            args: vec![
                Expression::Literal("5".to_string()),
                Expression::Literal("3".to_string()),
            ],
        };

        let result = Interpreter::eval_expression(&expr, &mut scope).unwrap();
        assert_eq!(result, Value::Integer(8));
    }
}
