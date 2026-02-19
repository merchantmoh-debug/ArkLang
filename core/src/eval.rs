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

use crate::ast::{ArkNode, Expression, Pattern, Statement};
use crate::runtime::{RuntimeError, Scope, Value};
use std::collections::HashSet;

pub struct Interpreter {
    recursion_limit: usize,
    current_depth: usize,
    imported_files: HashSet<String>,
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
            imported_files: HashSet::new(),
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
                        let val = self.eval_statement(stmt, scope);
                        match val {
                            Ok(Value::Return(v)) => return Ok(Value::Return(v)),
                            Ok(_) => {}
                            Err(RuntimeError::InvalidOperation(msg)) if msg == "BREAK" => {
                                return Ok(Value::Unit);
                            }
                            Err(RuntimeError::InvalidOperation(msg)) if msg == "CONTINUE" => break, // Break inner loop to continue while
                            Err(e) => return Err(e),
                        }
                    }
                }
                Ok(Value::Unit)
            }
            Statement::For {
                variable,
                iterable,
                body,
            } => {
                let iterable_val = self.eval_expression(iterable, scope)?;
                let items = match iterable_val {
                    Value::List(items) => items,
                    _ => return Err(RuntimeError::TypeMismatch("List".to_string(), iterable_val)),
                };

                for item in items {
                    scope.set(variable.clone(), item);
                    let mut broken = false;
                    for stmt in body {
                        let result = self.eval_statement(stmt, scope);
                        match result {
                            Ok(Value::Return(v)) => return Ok(Value::Return(v)),
                            Ok(_) => {}
                            Err(RuntimeError::InvalidOperation(msg)) if msg == "BREAK" => {
                                broken = true;
                                break;
                            }
                            Err(RuntimeError::InvalidOperation(msg)) if msg == "CONTINUE" => break, // Break inner loop (stmt loop) to continue outer loop (item loop)
                            Err(e) => return Err(e),
                        }
                    }
                    if broken {
                        break;
                    }
                }
                Ok(Value::Unit)
            }
            Statement::Break => Err(RuntimeError::InvalidOperation("BREAK".to_string())),
            Statement::Continue => Err(RuntimeError::InvalidOperation("CONTINUE".to_string())),
            Statement::Import(import_node) => {
                let path = &import_node.path;
                // Security Check: No path traversal
                if path.contains("..") {
                    return Err(RuntimeError::UntrustedCode);
                }

                if self.imported_files.contains(path) {
                    return Ok(Value::Unit);
                }
                self.imported_files.insert(path.clone());

                // Read file
                let content = std::fs::read_to_string(path).map_err(|_| {
                    RuntimeError::InvalidOperation(format!("Failed to read file: {}", path))
                })?;

                // Parse JSON (MAST)
                let node: ArkNode = serde_json::from_str(&content).map_err(|e| {
                    RuntimeError::InvalidOperation(format!("JSON Parse Error: {}", e))
                })?;

                self.eval(&node, scope)
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
            Statement::StructDecl(_)
            | Statement::EnumDecl(_)
            | Statement::TraitDecl(_)
            | Statement::ImplBlock(_) => {
                // Metadata-only declarations, not directly evaluated in tree-walker
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
            Expression::Match { scrutinee, arms } => {
                let val = self.eval_expression(scrutinee, scope)?;
                for (pattern, body) in arms {
                    // Create a child scope to isolate match arm bindings
                    let mut arm_scope = Scope::with_parent(scope);
                    if self.match_pattern(&val, pattern, &mut arm_scope) {
                        return self.eval_expression(body, &mut arm_scope);
                    }
                }
                Ok(Value::Unit)
            }
            Expression::Literal(s) => {
                // String Interpolation
                if s.contains('{') && s.contains('}') {
                    let mut result = String::new();
                    let mut chars = s.chars().peekable();
                    while let Some(c) = chars.next() {
                        if c == '{' {
                            // Check for double {{
                            if let Some(&next_c) = chars.peek() {
                                if next_c == '{' {
                                    chars.next(); // consume second {
                                    result.push('{');
                                    continue;
                                }
                            }

                            // Parse expression until }
                            let mut expr_str = String::new();
                            let mut closed = false;
                            while let Some(expr_c) = chars.next() {
                                if expr_c == '}' {
                                    closed = true;
                                    break;
                                }
                                expr_str.push(expr_c);
                            }

                            if !closed {
                                return Err(RuntimeError::InvalidOperation(
                                    "Unclosed string interpolation".to_string(),
                                ));
                            }

                            // Simple parser: variable or variable.field
                            let val = if expr_str.contains('.') {
                                let parts: Vec<&str> = expr_str.split('.').collect();
                                if parts.len() != 2 {
                                    // Fallback for complex expressions not supported without parser
                                    return Err(RuntimeError::InvalidOperation(format!(
                                        "Complex interpolation not supported: {}",
                                        expr_str
                                    )));
                                }
                                let var_name = parts[0].to_string();
                                let field_name = parts[1].to_string();

                                let obj_val = scope
                                    .get_or_move(&var_name)
                                    .ok_or_else(|| RuntimeError::VariableNotFound(var_name))?;

                                match obj_val {
                                    Value::Struct(data) => {
                                        data.get(&field_name).cloned().ok_or_else(|| {
                                            RuntimeError::VariableNotFound(field_name)
                                        })?
                                    }
                                    _ => {
                                        return Err(RuntimeError::TypeMismatch(
                                            "Struct".to_string(),
                                            obj_val,
                                        ));
                                    }
                                }
                            } else {
                                scope
                                    .get_or_move(&expr_str)
                                    .ok_or_else(|| RuntimeError::VariableNotFound(expr_str))?
                            };

                            // Convert val to string
                            let s_val = match val {
                                Value::String(s) => s,
                                Value::Integer(i) => i.to_string(),
                                Value::Boolean(b) => b.to_string(),
                                Value::Unit => "unit".to_string(),
                                _ => format!("{:?}", val),
                            };
                            result.push_str(&s_val);
                        } else {
                            result.push(c);
                        }
                    }
                    Ok(Value::String(result))
                } else {
                    // Standard Literal Parsing
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
            Expression::Integer(i) => Ok(Value::Integer(*i)),
            Expression::Lambda { params: _, body } => {
                // Evaluate lambda body inline (basic interpreter support)
                let mut last_val = Value::Unit;
                for stmt in body {
                    last_val = self.eval_statement(stmt, scope)?;
                    if let Value::Return(_) = last_val {
                        return Ok(last_val);
                    }
                }
                Ok(last_val)
            }
            Expression::EnumInit {
                enum_name,
                variant,
                args,
            } => {
                let mut fields = Vec::new();
                for arg in args {
                    fields.push(self.eval_expression(arg, scope)?);
                }
                Ok(Value::EnumValue {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    fields,
                })
            }
        }
    }

    fn match_pattern(&self, val: &Value, pattern: &Pattern, scope: &mut Scope) -> bool {
        match pattern {
            Pattern::Literal(s) => {
                if let Ok(i) = s.parse::<i64>() {
                    if let Value::Integer(v) = val {
                        return *v == i;
                    }
                }
                if s == "true" {
                    if let Value::Boolean(true) = val {
                        return true;
                    }
                }
                if s == "false" {
                    if let Value::Boolean(false) = val {
                        return true;
                    }
                }

                if let Value::String(v) = val {
                    return v == s;
                }
                false
            }
            Pattern::Variable(name) => {
                scope.set(name.clone(), val.clone());
                true
            }
            Pattern::Wildcard => true,
            Pattern::EnumVariant {
                enum_name: _,
                variant,
                bindings,
            } => {
                if let Value::EnumValue {
                    variant: v, fields, ..
                } = val
                {
                    if v == variant && fields.len() >= bindings.len() {
                        for (binding, field_val) in bindings.iter().zip(fields.iter()) {
                            scope.set(binding.clone(), field_val.clone());
                        }
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expression, Pattern};

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

    #[test]
    fn test_match_literal_patterns() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // match 10 { 5 => 0, 10 => 1 }
        let expr = Expression::Match {
            scrutinee: Box::new(Expression::Literal("10".to_string())),
            arms: vec![
                (
                    Pattern::Literal("5".to_string()),
                    Expression::Literal("0".to_string()),
                ),
                (
                    Pattern::Literal("10".to_string()),
                    Expression::Literal("1".to_string()),
                ),
            ],
        };

        let result = interpreter.eval_expression(&expr, &mut scope).unwrap();
        assert_eq!(result, Value::Integer(1));
    }

    #[test]
    fn test_match_wildcard() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // match 99 { 5 => 0, _ => 2 }
        let expr = Expression::Match {
            scrutinee: Box::new(Expression::Literal("99".to_string())),
            arms: vec![
                (
                    Pattern::Literal("5".to_string()),
                    Expression::Literal("0".to_string()),
                ),
                (Pattern::Wildcard, Expression::Literal("2".to_string())),
            ],
        };

        let result = interpreter.eval_expression(&expr, &mut scope).unwrap();
        assert_eq!(result, Value::Integer(2));
    }

    #[test]
    fn test_match_variable_binding() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // match 99 { x => x }
        let expr = Expression::Match {
            scrutinee: Box::new(Expression::Literal("99".to_string())),
            arms: vec![(
                Pattern::Variable("x".to_string()),
                Expression::Variable("x".to_string()),
            )],
        };

        let result = interpreter.eval_expression(&expr, &mut scope).unwrap();
        assert_eq!(result, Value::Integer(99));

        // Verify x is NOT in scope (scope isolation)
        assert!(scope.get("x").is_none());
    }

    #[test]
    fn test_for_loop_list() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // let res = []; for x in [1, 2] { res.append(x) }
        // We'll simulate by checking side effect on a variable 'sum'
        scope.set("sum".to_string(), Value::Integer(0));

        let stmt = Statement::For {
            variable: "x".to_string(),
            iterable: Expression::List(vec![
                Expression::Literal("1".to_string()),
                Expression::Literal("2".to_string()),
            ]),
            body: vec![Statement::Let {
                name: "sum".to_string(),
                ty: None,
                value: Expression::Call {
                    function_hash: "intrinsic_add".to_string(),
                    args: vec![
                        Expression::Variable("sum".to_string()),
                        Expression::Variable("x".to_string()),
                    ],
                },
            }],
        };

        interpreter.eval_statement(&stmt, &mut scope).unwrap();
        let sum = scope.get("sum").unwrap();
        assert_eq!(sum, Value::Integer(3));
    }

    #[test]
    fn test_for_loop_break() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();

        // let sum = 0; for x in [1, 2, 3] { if x == 2 { break; } sum = sum + x; }
        // sum should be 1.
        scope.set("sum".to_string(), Value::Integer(0));
        let stmt = Statement::For {
            variable: "x".to_string(),
            iterable: Expression::List(vec![
                Expression::Literal("1".to_string()),
                Expression::Literal("2".to_string()),
                Expression::Literal("3".to_string()),
            ]),
            body: vec![
                Statement::If {
                    condition: Expression::Call {
                        function_hash: "intrinsic_eq".to_string(),
                        args: vec![
                            Expression::Variable("x".to_string()),
                            Expression::Literal("2".to_string()),
                        ],
                    },
                    then_block: vec![Statement::Break],
                    else_block: None,
                },
                Statement::Let {
                    name: "sum".to_string(),
                    ty: None,
                    value: Expression::Call {
                        function_hash: "intrinsic_add".to_string(),
                        args: vec![
                            Expression::Variable("sum".to_string()),
                            Expression::Variable("x".to_string()),
                        ],
                    },
                },
            ],
        };
        interpreter.eval_statement(&stmt, &mut scope).unwrap();
        let sum = scope.get("sum").unwrap();
        assert_eq!(sum, Value::Integer(1));
    }

    #[test]
    fn test_string_interpolation() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();
        scope.set("name".to_string(), Value::String("Ark".to_string()));

        let expr = Expression::Literal("Hello {name}!".to_string());
        let result = interpreter.eval_expression(&expr, &mut scope).unwrap();
        assert_eq!(result, Value::String("Hello Ark!".to_string()));
    }

    #[test]
    fn test_error_on_undefined_variable() {
        let mut scope = Scope::new();
        let mut interpreter = Interpreter::new();
        let expr = Expression::Variable("undefined".to_string());
        let result = interpreter.eval_expression(&expr, &mut scope);
        assert!(matches!(result, Err(RuntimeError::VariableNotFound(_))));
    }
}
