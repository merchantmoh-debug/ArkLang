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
use crate::runtime::{RuntimeError, Scope, Value};

pub struct Interpreter {
    recursion_limit: usize,
    current_depth: usize,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            recursion_limit: 500,
            current_depth: 0,
        }
    }

    pub fn eval(&mut self, node: &ArkNode, scope: &mut Scope) -> Result<Value, RuntimeError> {
        self.current_depth += 1;
        if self.current_depth > self.recursion_limit {
            return Err(RuntimeError::RecursionLimitExceeded);
        }

        let result = match node {
            ArkNode::Statement(stmt) => self.eval_statement(stmt, scope),
            ArkNode::Expression(expr) => self.eval_expression(expr, scope),
            ArkNode::Function(func_def) => {
                // Interpreter deprecated for functions. VM is used.
                // We keep this for compatibility if referenced, but it does nothing useful for execution.
                scope.set(func_def.name.clone(), Value::Unit);
                Ok(Value::Unit)
            }
            _ => Ok(Value::Unit),
        };

        self.current_depth -= 1;

        match result {
            Ok(Value::Return(val)) => Ok(*val),
            other => other,
        }
    }

    fn eval_statement(
        &mut self,
        stmt: &Statement,
        scope: &mut Scope,
    ) -> Result<Value, RuntimeError> {
        match stmt {
            Statement::Let { name, ty: _, value } => {
                let val = self.eval_expression(value, scope)?;
                scope.set(name.clone(), val);
                Ok(Value::Unit)
            }
            Statement::LetDestructure { names, value } => {
                let result = self.eval_expression(value, scope)?;
                match result {
                    Value::List(items) => {
                        if items.len() != names.len() {
                            println!(
                                "Destructuring mismatch: expected {} items, got {}",
                                names.len(),
                                items.len()
                            );
                            return Err(RuntimeError::NotExecutable);
                        }
                        for (i, val) in items.into_iter().enumerate() {
                            scope.set(names[i].clone(), val);
                        }
                        Ok(Value::Unit)
                    }
                    _ => Err(RuntimeError::TypeMismatch("List".to_string(), result)),
                }
            }
            Statement::Return(expr) => {
                let val = self.eval_expression(expr, scope)?;
                Ok(Value::Return(Box::new(val)))
            }
            Statement::Block(stmts) => {
                let mut last_val = Value::Unit;
                for stmt in stmts {
                    let val = self.eval_statement(stmt, scope)?;
                    if let Value::Return(_) = val {
                        return Ok(val);
                    }
                    last_val = val;
                }
                Ok(last_val)
            }
            Statement::Expression(expr) => self.eval_expression(expr, scope),
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = self.eval_expression(condition, scope)?;
                let is_true = match cond_val {
                    Value::Integer(i) => i != 0,
                    Value::Boolean(b) => b,
                    _ => false,
                };

                let mut result = Value::Unit;
                if is_true {
                    for stmt in then_block {
                        result = self.eval_statement(stmt, scope)?;
                        if let Value::Return(_) = result {
                            return Ok(result);
                        }
                    }
                } else if let Some(stmts) = else_block {
                    for stmt in stmts {
                        result = self.eval_statement(stmt, scope)?;
                        if let Value::Return(_) = result {
                            return Ok(result);
                        }
                    }
                }
                Ok(result)
            }
            Statement::While { condition, body } => {
                loop {
                    let cond_val = self.eval_expression(condition, scope)?;
                    let is_true = match cond_val {
                        Value::Integer(i) => i != 0,
                        Value::Boolean(b) => b,
                        _ => false,
                    };

                    if !is_true {
                        break;
                    }

                    for stmt in body {
                        let val = self.eval_statement(stmt, scope)?;
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
                let val = self.eval_expression(value, scope)?;
                let mut obj = scope
                    .take(obj_name)
                    .ok_or_else(|| RuntimeError::VariableNotFound(obj_name.clone()))?;

                match &mut obj {
                    Value::Struct(fields) => {
                        fields.insert(field.clone(), val);
                    }
                    _ => return Err(RuntimeError::TypeMismatch("Struct".to_string(), obj)),
                }
                scope.set(obj_name.clone(), obj);
                Ok(Value::Unit)
            }
            Statement::Function(func_def) => {
                scope.set(func_def.name.clone(), Value::Unit);
                Ok(Value::Unit)
            }
            Statement::Import(_) | Statement::StructDecl(_) => {
                println!("Interpreter Warning: New AST nodes Import/StructDecl not supported in tree-walker.");
                Ok(Value::Unit)
            }
        }
    }

    fn eval_expression(
        &mut self,
        expr: &Expression,
        scope: &mut Scope,
    ) -> Result<Value, RuntimeError> {
        self.current_depth += 1;
        if self.current_depth > self.recursion_limit {
            return Err(RuntimeError::RecursionLimitExceeded);
        }

        let result = self.eval_expression_impl(expr, scope);

        self.current_depth -= 1;
        result
    }

    fn eval_expression_impl(
        &mut self,
        expr: &Expression,
        scope: &mut Scope,
    ) -> Result<Value, RuntimeError> {
        match expr {
            Expression::StructInit { fields } => {
                let mut data = std::collections::HashMap::new();
                for (name, expr) in fields {
                    let val = self.eval_expression(expr, scope)?;
                    data.insert(name.clone(), val);
                }
                Ok(Value::Struct(data))
            }
            Expression::GetField { obj, field } => {
                let obj_val = self.eval_expression(obj, scope)?;
                match obj_val {
                    Value::Struct(mut data) => data
                        .remove(field)
                        .ok_or_else(|| RuntimeError::VariableNotFound(field.clone())),
                    _ => Err(RuntimeError::TypeMismatch("Struct".to_string(), obj_val)),
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
                .ok_or_else(|| RuntimeError::VariableNotFound(name.clone())),
            Expression::Call {
                function_hash,
                args,
            } => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expression(arg, scope)?);
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
                        println!(
                            "Interpreter Warning: Cannot execute bytecode function '{}' in tree-walker.",
                            function_hash
                        );
                        return Err(RuntimeError::NotExecutable);
                    } else {
                        println!(
                            "Found variable '{}' but it is not a function: {:?}",
                            function_hash, val
                        );
                    }
                } else {
                    println!("Function '{}' not found in scope.", function_hash);
                }

                Err(RuntimeError::NotExecutable)
            }
            Expression::List(items) => {
                let mut values = Vec::new();
                for item in items {
                    values.push(self.eval_expression(item, scope)?);
                }
                Ok(Value::List(values))
            }
            Expression::Match(_) | Expression::Lambda(_) | Expression::TryCatch(_) => {
                println!("Interpreter Warning: New AST nodes Match/Lambda/TryCatch not supported in tree-walker.");
                Err(RuntimeError::NotExecutable)
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
        let mut interpreter = Interpreter::new();

        // 5 + 3
        let expr = Expression::Call {
            function_hash: "intrinsic_add".to_string(),
            args: vec![
                Expression::Literal("5".to_string()),
                Expression::Literal("3".to_string()),
            ],
        };

        let result = interpreter.eval_expression(&expr, &mut scope).unwrap();
        assert_eq!(result, Value::Integer(8));
    }

    #[test]
    fn test_recursion_limit() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();
        interpreter.recursion_limit = 5;

        // Create a deep chain of calls: add(add(add(...)))
        let mut expr = Expression::Literal("1".to_string());
        for _ in 0..10 {
            expr = Expression::Call {
                function_hash: "intrinsic_add".to_string(),
                args: vec![expr, Expression::Literal("1".to_string())],
            };
        }

        let result = interpreter.eval_expression(&expr, &mut scope);
        assert!(matches!(result, Err(RuntimeError::RecursionLimitExceeded)));
    }

    #[test]
    fn test_destructuring_mismatch_not_enough_values() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // let (a, b) = [1];
        let stmt = Statement::LetDestructure {
            names: vec!["a".to_string(), "b".to_string()],
            value: Expression::List(vec![Expression::Literal("1".to_string())]),
        };

        let result = interpreter.eval_statement(&stmt, &mut scope);
        assert!(matches!(result, Err(RuntimeError::NotExecutable)));
    }

    #[test]
    fn test_destructuring_mismatch_too_many_values() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // let (a) = [1, 2];
        let stmt = Statement::LetDestructure {
            names: vec!["a".to_string()],
            value: Expression::List(vec![
                Expression::Literal("1".to_string()),
                Expression::Literal("2".to_string()),
            ]),
        };

        let result = interpreter.eval_statement(&stmt, &mut scope);
        assert!(matches!(result, Err(RuntimeError::NotExecutable)));
    }

    #[test]
    fn test_destructuring_success() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // let (a, b) = [1, 2];
        let stmt = Statement::LetDestructure {
            names: vec!["a".to_string(), "b".to_string()],
            value: Expression::List(vec![
                Expression::Literal("1".to_string()),
                Expression::Literal("2".to_string()),
            ]),
        };

        let result = interpreter.eval_statement(&stmt, &mut scope);
        assert!(matches!(result, Ok(Value::Unit)));

        // Verify variables are set
        let a = scope.get_or_move(&"a".to_string()).unwrap();
        let b = scope.get_or_move(&"b".to_string()).unwrap();
        assert_eq!(a, Value::Integer(1));
        assert_eq!(b, Value::Integer(2));
    }
}
