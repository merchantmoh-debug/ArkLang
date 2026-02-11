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
        let result = match node {
            ArkNode::Statement(stmt) => Interpreter::eval_statement(stmt, scope),
            ArkNode::Expression(expr) => Interpreter::eval_expression(expr, scope),
            ArkNode::Function(func_def) => {
                // Interpreter deprecated for functions. VM is used.
                scope.set(func_def.name.clone(), Value::Unit);
                Ok(Value::Unit)
            }
            _ => Ok(Value::Unit),
        };

        match result {
            Ok(Value::Return(val)) => Ok(*val),
            other => other,
        }
    }

    fn eval_statement(stmt: &Statement, scope: &mut Scope) -> Result<Value, EvalError> {
        match stmt {
            Statement::Let { name, ty: _, value } => {
                let val = Interpreter::eval_expression(value, scope)?;
                scope.set(name.clone(), val);
                Ok(Value::Unit)
            }
            Statement::LetDestructure { names, value } => {
                let result = Interpreter::eval_expression(value, scope)?;
                match result {
                    Value::List(items) => {
                        if items.len() != names.len() {
                            // Should be runtime error
                            println!(
                                "Destructuring mismatch: expected {} items, got {}",
                                names.len(),
                                items.len()
                            );
                            return Err(EvalError::NotExecutable);
                        }
                        for (i, val) in items.into_iter().enumerate() {
                            scope.set(names[i].clone(), val);
                        }
                        Ok(Value::Unit)
                    }
                    _ => Err(EvalError::TypeMismatch("List".to_string(), result)),
                }
            }
            Statement::Return(expr) => {
                let val = Interpreter::eval_expression(expr, scope)?;
                Ok(Value::Return(Box::new(val)))
            }
            Statement::Block(stmts) => {
                let mut last_val = Value::Unit;
                for stmt in stmts {
                    let val = Interpreter::eval_statement(stmt, scope)?;
                    if let Value::Return(_) = val {
                        return Ok(val);
                    }
                    last_val = val;
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
                        if let Value::Return(_) = result {
                            return Ok(result);
                        }
                    }
                } else if let Some(stmts) = else_block {
                    for stmt in stmts {
                        result = Interpreter::eval_statement(stmt, scope)?;
                        if let Value::Return(_) = result {
                            return Ok(result);
                        }
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
                        let val = Interpreter::eval_statement(stmt, scope)?;
                        if let Value::Return(_) = val {
                            return Ok(val);
                        }
                    }
                }
                Ok(Value::Unit)
            }
            Statement::SetField {
                obj_name,
                field,
                value,
            } => {
                let val = Interpreter::eval_expression(value, scope)?;
                let mut obj = scope
                    .take(obj_name)
                    .ok_or_else(|| EvalError::VariableNotFound(obj_name.clone()))?;

                match &mut obj {
                    Value::Struct(fields) => {
                        fields.insert(field.clone(), val);
                    }
                    _ => return Err(EvalError::TypeMismatch("Struct".to_string(), obj)),
                }
                scope.set(obj_name.clone(), obj);
                Ok(Value::Unit)
            }
            Statement::Function(func_def) => {
                scope.set(func_def.name.clone(), Value::Unit);
                Ok(Value::Unit)
            }
        }
    }

    fn eval_expression(expr: &Expression, scope: &mut Scope) -> Result<Value, EvalError> {
        match expr {
            Expression::StructInit { fields } => {
                let mut data = std::collections::HashMap::new();
                for (name, expr) in fields {
                    let val = Interpreter::eval_expression(expr, scope)?;
                    data.insert(name.clone(), val);
                }
                Ok(Value::Struct(data))
            }
            Expression::GetField { obj, field } => {
                let obj_val = Interpreter::eval_expression(obj, scope)?;
                match obj_val {
                    Value::Struct(mut data) => data
                        .remove(field)
                        .ok_or_else(|| EvalError::VariableNotFound(field.clone())),
                    _ => Err(EvalError::TypeMismatch("Struct".to_string(), obj_val)),
                }
            }
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
                .get_or_move(name)
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
                if let Some(val) = scope.get_or_move(function_hash) {
                    if let Value::Function(_) = val {
                        // Interpreter cannot execute Bytecode Functions directly.
                        // This path should only be taken if running pure Interpreter mode, which is deprecated for functions.
                        println!(
                            "Interpreter Warning: Cannot execute bytecode function '{}' in tree-walker.",
                            function_hash
                        );
                        return Err(EvalError::NotExecutable);
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
            Expression::List(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(Interpreter::eval_expression(item, scope)?);
                }
                Ok(Value::List(values))
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
