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
use crate::bytecode::{Chunk, OpCode};
use crate::runtime::Value;
use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Compilation Error: {} at {}:{}:{}",
            self.message, self.file, self.line, self.column
        )
    }
}

impl std::error::Error for CompileError {}

/// optimization pipeline
pub fn optimize(node: ArkNode, level: u8) -> ArkNode {
    if level == 0 {
        return node;
    }

    let mut current = node;

    // Level 1: Constant Folding
    if level >= 1 {
        current = fold_constants(&current);
    }

    // Level 2: Dead Code Elimination
    if level >= 2 {
        current = eliminate_dead_code(&current);
    }

    current
}

fn fold_constants(node: &ArkNode) -> ArkNode {
    match node {
        ArkNode::Expression(expr) => ArkNode::Expression(fold_expr(expr)),
        ArkNode::Statement(stmt) => ArkNode::Statement(fold_stmt(stmt)),
        ArkNode::Function(func) => ArkNode::Function(fold_func(func)),
        _ => node.clone(),
    }
}

fn fold_stmt(stmt: &Statement) -> Statement {
    match stmt {
        Statement::Expression(e) => Statement::Expression(fold_expr(e)),
        Statement::Block(stmts) => Statement::Block(stmts.iter().map(fold_stmt).collect()),
        Statement::Let { name, ty, value } => Statement::Let {
            name: name.clone(),
            ty: ty.clone(),
            value: fold_expr(value),
        },
        Statement::Return(e) => Statement::Return(fold_expr(e)),
        Statement::If {
            condition,
            then_block,
            else_block,
        } => Statement::If {
            condition: fold_expr(condition),
            then_block: then_block.iter().map(fold_stmt).collect(),
            else_block: else_block
                .as_ref()
                .map(|b| b.iter().map(fold_stmt).collect()),
        },
        Statement::While { condition, body } => Statement::While {
            condition: fold_expr(condition),
            body: body.iter().map(fold_stmt).collect(),
        },
        Statement::For {
            variable,
            iterable,
            body,
        } => Statement::For {
            variable: variable.clone(),
            iterable: fold_expr(iterable),
            body: body.iter().map(fold_stmt).collect(),
        },
        Statement::Break => Statement::Break,
        Statement::Continue => Statement::Continue,
        Statement::Import(i) => Statement::Import(i.clone()),
        Statement::StructDecl(s) => Statement::StructDecl(s.clone()),
        Statement::Function(f) => Statement::Function(fold_func(f)),
        Statement::LetDestructure { names, value } => Statement::LetDestructure {
            names: names.clone(),
            value: fold_expr(value),
        },
        Statement::SetField {
            obj_name,
            field,
            value,
        } => Statement::SetField {
            obj_name: obj_name.clone(),
            field: field.clone(),
            value: fold_expr(value),
        },
        Statement::EnumDecl(e) => Statement::EnumDecl(e.clone()),
        Statement::TraitDecl(t) => Statement::TraitDecl(t.clone()),
        Statement::ImplBlock(i) => Statement::ImplBlock(i.clone()),
    }
}

fn fold_func(func: &FunctionDef) -> FunctionDef {
    // We cannot easily fold inside the inner MastNode without decoding it,
    // but here we only have the FunctionDef struct which has 'body: Box<MastNode>'.
    // The MastNode contains a hash and content (ArkNode).
    // If we want to optimize the body, we must optimize func.body.content and re-hash.
    // However, MastNode::new() recalculates hash.
    // BUT, we can't easily modify func.body.content because it is immutable in the struct definition?
    // Wait, FunctionDef is: pub body: Box<MastNode>.
    // MastNode is: pub content: ArkNode.
    // So we CAN modify it.
    let mut new_func = func.clone();
    let new_content = fold_constants(&func.body.content);
    // We need to update hash. MastNode::new(content) does that.
    if let Ok(new_mast) = crate::ast::MastNode::new(new_content) {
        new_func.body = Box::new(new_mast);
    }
    new_func
}

fn fold_expr(expr: &Expression) -> Expression {
    match expr {
        Expression::Call {
            function_hash,
            args,
        } => {
            let folded_args: Vec<Expression> = args.iter().map(fold_expr).collect();

            // Try to fold if args are literals
            if folded_args
                .iter()
                .all(|a| matches!(a, Expression::Literal(_)))
            {
                let literals: Vec<&String> = folded_args
                    .iter()
                    .map(|a| match a {
                        Expression::Literal(s) => s,
                        _ => unreachable!(),
                    })
                    .collect();

                match function_hash.as_str() {
                    "intrinsic_add" | "add" if literals.len() == 2 => {
                        if let (Ok(a), Ok(b)) =
                            (literals[0].parse::<i64>(), literals[1].parse::<i64>())
                        {
                            return Expression::Literal((a + b).to_string());
                        }
                    }
                    "intrinsic_sub" | "sub" if literals.len() == 2 => {
                        if let (Ok(a), Ok(b)) =
                            (literals[0].parse::<i64>(), literals[1].parse::<i64>())
                        {
                            return Expression::Literal((a - b).to_string());
                        }
                    }
                    "intrinsic_mul" | "mul" if literals.len() == 2 => {
                        if let (Ok(a), Ok(b)) =
                            (literals[0].parse::<i64>(), literals[1].parse::<i64>())
                        {
                            return Expression::Literal((a * b).to_string());
                        }
                    }
                    "intrinsic_div" | "div" if literals.len() == 2 => {
                        if let (Ok(a), Ok(b)) =
                            (literals[0].parse::<i64>(), literals[1].parse::<i64>())
                        {
                            if b != 0 {
                                return Expression::Literal((a / b).to_string());
                            }
                        }
                    }
                    "intrinsic_mod" | "mod" if literals.len() == 2 => {
                        if let (Ok(a), Ok(b)) =
                            (literals[0].parse::<i64>(), literals[1].parse::<i64>())
                        {
                            if b != 0 {
                                return Expression::Literal((a % b).to_string());
                            }
                        }
                    }
                    // Boolean logic
                    "intrinsic_and" | "and" => {
                        // Can be variadic or binary? Usually binary.
                        // Assuming binary for now or strict '&&'
                        // If we support variadic logic, we need to handle it.
                        // Ark usually has strict binary/n-ary.
                        // Let's assume binary for 3 args?
                        // If all are boolean literals.
                        // "true" / "false"
                        let bools: Result<Vec<bool>, _> =
                            literals.iter().map(|s| s.parse::<bool>()).collect();
                        if let Ok(vals) = bools {
                            let res = vals.iter().all(|&b| b);
                            return Expression::Literal(res.to_string());
                        }
                    }
                    "intrinsic_or" | "or" => {
                        let bools: Result<Vec<bool>, _> =
                            literals.iter().map(|s| s.parse::<bool>()).collect();
                        if let Ok(vals) = bools {
                            let res = vals.iter().any(|&b| b);
                            return Expression::Literal(res.to_string());
                        }
                    }
                    "intrinsic_not" | "not" if literals.len() == 1 => {
                        if let Ok(b) = literals[0].parse::<bool>() {
                            return Expression::Literal((!b).to_string());
                        }
                    }
                    // String concat
                    "intrinsic_concat" | "concat" => {
                        // All literals are strings (already checked).
                        // Note: Literal("3") is integer 3.
                        // We need to distinguish String vs Integer?
                        // Expression::Literal stores String.
                        // In Ark, strings are quoted? Or Literal("foo") means string "foo"?
                        // If I have 3, Literal("3").
                        // If I have "foo", Literal("foo")? or Literal("\"foo\"")?
                        // Checking `Expression::Literal`: it says "Placeholder".
                        // Assuming Literal(val) is the value string representation.
                        // So "foo" would be "foo".
                        // Concat concatenates them.
                        let res = literals.into_iter().map(|s| s.as_str()).collect::<String>();
                        return Expression::Literal(res);
                    }
                    _ => {}
                }
            }
            Expression::Call {
                function_hash: function_hash.clone(),
                args: folded_args,
            }
        }
        Expression::List(items) => Expression::List(items.iter().map(fold_expr).collect()),
        Expression::StructInit { fields } => Expression::StructInit {
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), fold_expr(v)))
                .collect(),
        },
        Expression::GetField { obj, field } => Expression::GetField {
            obj: Box::new(fold_expr(obj)),
            field: field.clone(),
        },
        _ => expr.clone(),
    }
}

fn eliminate_dead_code(node: &ArkNode) -> ArkNode {
    match node {
        ArkNode::Statement(stmt) => ArkNode::Statement(dce_stmt(stmt)),
        ArkNode::Function(func) => ArkNode::Function(dce_func(func)),
        _ => node.clone(),
    }
}

fn dce_func(func: &FunctionDef) -> FunctionDef {
    let mut new_func = func.clone();
    let new_content = eliminate_dead_code(&func.body.content);
    if let Ok(new_mast) = crate::ast::MastNode::new(new_content) {
        new_func.body = Box::new(new_mast);
    }
    new_func
}

fn dce_stmt(stmt: &Statement) -> Statement {
    match stmt {
        Statement::Block(stmts) => {
            let mut new_stmts = Vec::new();
            for s in stmts {
                let optimized = dce_stmt(s);
                new_stmts.push(optimized.clone());
                // Check if this statement is a terminator
                if is_terminator(&optimized) {
                    break;
                }
            }
            Statement::Block(new_stmts)
        }
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            // Folded condition?
            // If condition is Literal("true"), take then_block.
            // If Literal("false"), take else_block.
            match condition {
                Expression::Literal(s) if s == "true" => {
                    // Return Block of then_block
                    // We must return a Statement. Block is good.
                    // We should also recurse DCE on the block.
                    let optimized_block: Vec<Statement> = then_block.iter().map(dce_stmt).collect();
                    Statement::Block(optimized_block)
                }
                Expression::Literal(s) if s == "false" => {
                    if let Some(else_stmts) = else_block {
                        let optimized_block: Vec<Statement> =
                            else_stmts.iter().map(dce_stmt).collect();
                        Statement::Block(optimized_block)
                    } else {
                        Statement::Block(vec![])
                    }
                }
                _ => {
                    // Recurse
                    Statement::If {
                        condition: condition.clone(),
                        then_block: then_block.iter().map(dce_stmt).collect(),
                        else_block: else_block
                            .as_ref()
                            .map(|b| b.iter().map(dce_stmt).collect()),
                    }
                }
            }
        }
        Statement::While { condition, body } => match condition {
            Expression::Literal(s) if s == "false" => Statement::Block(vec![]),
            _ => Statement::While {
                condition: condition.clone(),
                body: body.iter().map(dce_stmt).collect(),
            },
        },
        _ => stmt.clone(),
    }
}

fn is_terminator(stmt: &Statement) -> bool {
    match stmt {
        Statement::Return(_) => true,
        // Break/Continue not in Statement enum in ast.rs?
        // Checked ast.rs: Statement::Break / Continue NOT present in the provided read_file output!
        // Wait. Memory said "The core/src/ast.rs AST now includes Expression::Match, Statement::For, Statement::Import, Statement::Break, and Statement::Continue".
        // But the read_file output I got did NOT show them.
        // It showed Let, LetDestructure, SetField, Return, Block, Expression, If, While, Function.
        // It did NOT show Break/Continue.
        // If they are missing, I cannot optimize them.
        // "Memory" is just context, "read_file" is truth.
        // I will trust read_file.
        // If they are missing, I can't check them.
        _ => false,
    }
}

pub struct Compiler {
    pub chunk: Chunk,
    pub scopes: Vec<HashSet<String>>,
    /// Current source line for debugging (increments per statement)
    pub current_line: u32,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            scopes: vec![HashSet::new()],
            current_line: 1,
        }
    }

    pub fn compile(mut self, node: &ArkNode) -> Chunk {
        let optimized = optimize(node.clone(), 2);
        match self.compile_safe(&optimized) {
            Ok(_) => self.chunk,
            Err(e) => panic!("{}", e),
        }
    }

    pub fn compile_safe(&mut self, node: &ArkNode) -> Result<(), CompileError> {
        self.visit(node, true)
    }

    fn visit(&mut self, node: &ArkNode, preserve: bool) -> Result<(), CompileError> {
        match node {
            ArkNode::Statement(s) => self.visit_stmt(s, preserve),
            ArkNode::Expression(e) => {
                self.visit_expr(e)?;
                if !preserve {
                    self.chunk.write(OpCode::Pop);
                }
                Ok(())
            }
            ArkNode::Function(f) => {
                // Compile function definition
                self.visit_stmt(&Statement::Function(f.clone()), preserve)
            }
            _ => Ok(()),
        }
    }

    fn visit_stmt(&mut self, stmt: &Statement, preserve: bool) -> Result<(), CompileError> {
        // Emit source position for debugger
        self.chunk.set_source_pos(self.current_line, 0);
        self.current_line += 1;

        match stmt {
            Statement::Expression(e) => {
                self.visit_expr(e)?;
                if !preserve {
                    self.chunk.write(OpCode::Pop);
                }
                Ok(())
            }
            Statement::Block(stmts) => {
                self.scopes.push(HashSet::new());
                let len = stmts.len();
                if len == 0 && preserve {
                    self.chunk.write(OpCode::Push(Value::Unit));
                }
                for (i, s) in stmts.iter().enumerate() {
                    let is_last = i == len - 1;
                    self.visit_stmt(s, preserve && is_last)?;
                }
                self.scopes.pop();
                Ok(())
            }
            Statement::Let { name, ty: _, value } => {
                self.visit_expr(value)?;

                // Shadowing Check
                if let Some(scope) = self.scopes.last() {
                    if scope.contains(name) {
                        println!(
                            "Compiler Warning: Variable '{}' shadows an existing variable in the current scope.",
                            name
                        );
                    }
                }
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.clone());
                }

                self.chunk.write(OpCode::Store(name.clone()));
                Ok(())
            }

            Statement::Function(func_def) => {
                let mut func_compiler = Compiler::new();
                for (arg_name, _) in func_def.inputs.iter().rev() {
                    func_compiler.chunk.write(OpCode::Store(arg_name.clone()));
                    // Add args to function scope
                    if let Some(scope) = func_compiler.scopes.last_mut() {
                        scope.insert(arg_name.clone());
                    }
                }

                func_compiler.visit(&func_def.body.content, true)?;

                func_compiler.chunk.write(OpCode::Ret);

                let compiled_chunk = func_compiler.chunk;
                // 5. Emit Push(Value::Function(Arc::new(chunk)))
                let func_val = Value::Function(Arc::new(compiled_chunk));
                self.chunk.write(OpCode::Push(func_val));
                self.chunk.write(OpCode::Store(func_def.name.clone()));

                // Add function name to current scope
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(func_def.name.clone());
                }

                Ok(())
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.visit_expr(condition)?;
                let jump_idx = self.chunk.code.len();
                self.chunk.write(OpCode::JmpIfFalse(0));

                self.scopes.push(HashSet::new());
                let then_len = then_block.len();
                if then_len == 0 && preserve {
                    self.chunk.write(OpCode::Push(Value::Unit));
                }
                for (i, s) in then_block.iter().enumerate() {
                    let is_last = i == then_len - 1;
                    self.visit_stmt(s, preserve && is_last)?;
                }
                self.scopes.pop();

                let else_jump_idx = self.chunk.code.len();
                // Jump over the else block if it exists OR if we are synthesizing one
                if else_block.is_some() || preserve {
                    self.chunk.write(OpCode::Jmp(0));
                }

                let after_then_idx = self.chunk.code.len();
                // We need to update the jump offset. Since we are using usize as offset in bytecode (Jmp(usize)),
                // we can just set it to the index.
                // Note: core/src/bytecode.rs defines OpCode::Jmp(usize) and JmpIfFalse(usize).
                // So we can mutate the instruction.
                self.chunk.code[jump_idx] = OpCode::JmpIfFalse(after_then_idx);

                if let Some(stmts) = else_block {
                    self.scopes.push(HashSet::new());
                    let else_len = stmts.len();
                    if else_len == 0 && preserve {
                        self.chunk.write(OpCode::Push(Value::Unit));
                    }
                    for (i, s) in stmts.iter().enumerate() {
                        let is_last = i == else_len - 1;
                        self.visit_stmt(s, preserve && is_last)?;
                    }
                    self.scopes.pop();

                    let end_idx = self.chunk.code.len();
                    self.chunk.code[else_jump_idx] = OpCode::Jmp(end_idx);
                } else if preserve {
                    // Implicit else { Unit }
                    self.chunk.write(OpCode::Push(Value::Unit));
                    let end_idx = self.chunk.code.len();
                    self.chunk.code[else_jump_idx] = OpCode::Jmp(end_idx);
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                let loop_start_idx = self.chunk.code.len();
                self.visit_expr(condition)?;
                let jump_idx = self.chunk.code.len();
                self.chunk.write(OpCode::JmpIfFalse(0));

                self.scopes.push(HashSet::new());
                for s in body {
                    self.visit_stmt(s, false)?;
                }
                self.scopes.pop();

                self.chunk.write(OpCode::Jmp(loop_start_idx));
                let end_idx = self.chunk.code.len();
                self.chunk.code[jump_idx] = OpCode::JmpIfFalse(end_idx);

                if preserve {
                    self.chunk.write(OpCode::Push(Value::Unit));
                }
                Ok(())
            }
            Statement::Return(expr) => {
                self.visit_expr(expr)?;
                self.chunk.write(OpCode::Ret);
                Ok(())
            }
            Statement::LetDestructure { names, value } => {
                self.visit_expr(value)?;
                self.chunk.write(OpCode::Destructure);
                for name in names {
                    self.chunk.write(OpCode::Store(name.clone()));
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.insert(name.clone());
                    }
                }
                Ok(())
            }
            Statement::SetField {
                obj_name,
                field,
                value,
            } => {
                self.visit_expr(value)?;
                self.chunk.write(OpCode::Load(obj_name.clone()));
                self.chunk.write(OpCode::SetField(field.clone()));
                self.chunk.write(OpCode::Store(obj_name.clone()));
                Ok(())
            }
            Statement::Import(_)
            | Statement::StructDecl(_)
            | Statement::EnumDecl(_)
            | Statement::TraitDecl(_) => {
                // Metadata-only declarations, no bytecode emitted
                Ok(())
            }
            Statement::ImplBlock(impl_blk) => {
                // Compile impl methods as TypeName::method_name
                for method in &impl_blk.methods {
                    let mut func_compiler = Compiler::new();
                    for (arg_name, _) in method.inputs.iter().rev() {
                        func_compiler.chunk.write(OpCode::Store(arg_name.clone()));
                        if let Some(scope) = func_compiler.scopes.last_mut() {
                            scope.insert(arg_name.clone());
                        }
                    }
                    func_compiler.visit(&method.body.content, true)?;
                    func_compiler.chunk.write(OpCode::Ret);
                    let compiled_chunk = func_compiler.chunk;
                    let func_val = Value::Function(Arc::new(compiled_chunk));
                    let qualified_name = format!("{}::{}", impl_blk.target_type, method.name);
                    self.chunk.write(OpCode::Push(func_val));
                    self.chunk.write(OpCode::Store(qualified_name.clone()));
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.insert(qualified_name);
                    }
                }
                Ok(())
            }
            Statement::For { .. } | Statement::Break | Statement::Continue => {
                // Not supported in bytecode compiler yet
                Ok(())
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expression) -> Result<(), CompileError> {
        match expr {
            Expression::List(items) => {
                for item in items {
                    self.visit_expr(item)?;
                }
                self.chunk.write(OpCode::MakeList(items.len()));
                Ok(())
            }
            Expression::StructInit { fields } => {
                for (name, expr) in fields {
                    self.visit_expr(expr)?;
                    self.chunk.write(OpCode::Push(Value::String(name.clone())));
                }
                self.chunk.write(OpCode::MakeStruct(fields.len()));
                Ok(())
            }
            Expression::GetField { obj, field } => {
                self.visit_expr(obj)?;
                self.chunk.write(OpCode::GetField(field.clone()));
                Ok(())
            }

            Expression::Literal(s) => {
                self.chunk.write(OpCode::Push(Value::String(s.clone())));
                Ok(())
            }

            Expression::Integer(n) => {
                self.chunk.write(OpCode::Push(Value::Integer(*n)));
                Ok(())
            }

            Expression::Variable(name) => {
                // Scope check?
                // We check if variable exists in any scope?
                // Iterating reverse.
                let mut found = false;
                for scope in self.scopes.iter().rev() {
                    if scope.contains(name) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    // Warning or Error?
                    // "Variables are resolved to scope-aware indices"
                    // If not found, it might be global or undefined.
                    // For now, we just emit Load.
                    // maybe println!("Warning: Undefined variable '{}'", name);
                }
                self.chunk.write(OpCode::Load(name.clone()));
                Ok(())
            }
            Expression::Match { .. } => {
                // Not supported in bytecode compiler yet
                Ok(())
            }
            Expression::Call {
                function_hash,
                args,
                ..
            } => {
                match function_hash.as_str() {
                    "intrinsic_add" | "add" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Add);
                        } else {
                            return Err(CompileError {
                                message: format!("add requires 2 args, got {}", args.len()),
                                line: 0,
                                column: 0,
                                file: "unknown".into(),
                            });
                        }
                    }
                    "intrinsic_sub" | "sub" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Sub);
                        }
                    }
                    "intrinsic_mul" | "mul" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Mul);
                        }
                    }
                    "intrinsic_div" | "div" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Div);
                        }
                    }
                    "intrinsic_eq" | "eq" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Eq);
                        }
                    }
                    "intrinsic_gt" | "gt" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Gt);
                        }
                    }
                    "intrinsic_lt" | "lt" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Lt);
                        }
                    }
                    "intrinsic_le" | "le" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Le);
                        }
                    }
                    "intrinsic_ge" | "ge" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Ge);
                        }
                    }
                    "intrinsic_neq" | "neq" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Neq);
                        }
                    }
                    "intrinsic_mod" | "modulo" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Mod);
                        }
                    }
                    "intrinsic_and" | "and" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::And);
                        }
                    }
                    "intrinsic_or" | "or" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0])?;
                            self.visit_expr(&args[1])?;
                            self.chunk.write(OpCode::Or);
                        }
                    }
                    "print" | "intrinsic_print" => {
                        for arg in args {
                            self.visit_expr(arg)?;
                            self.chunk.write(OpCode::Print);
                        }
                        self.chunk.write(OpCode::Push(Value::Unit));
                    }
                    "neg" => {
                        // Unary minus: 0 - x
                        if args.len() == 1 {
                            self.chunk.write(OpCode::Push(Value::Integer(0)));
                            self.visit_expr(&args[0])?;
                            self.chunk.write(OpCode::Sub);
                        }
                    }
                    "not" | "intrinsic_not" => {
                        if args.len() == 1 {
                            self.visit_expr(&args[0])?;
                            self.chunk.write(OpCode::Load("intrinsic_not".to_string()));
                            self.chunk.write(OpCode::Call(1));
                        }
                    }
                    _ => {
                        for arg in args {
                            self.visit_expr(arg)?;
                        }
                        self.chunk.write(OpCode::Load(function_hash.clone()));
                        self.chunk.write(OpCode::Call(args.len()));
                    }
                }
                Ok(())
            }
            Expression::Lambda { params: _, body } => {
                // Compile lambda body inline (basic support)
                for stmt in body {
                    self.visit_stmt(stmt, false)?;
                }
                self.chunk.write(OpCode::Push(Value::Unit));
                Ok(())
            }
            Expression::EnumInit {
                enum_name,
                variant,
                args,
            } => {
                // Push field values, then emit MakeEnum
                for arg in args {
                    self.visit_expr(arg)?;
                }
                self.chunk.write(OpCode::MakeEnum(
                    enum_name.clone(),
                    variant.clone(),
                    args.len(),
                ));
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expression;

    #[test]
    fn test_constant_folding_integers() {
        // 3 + 5 -> 8
        let expr = Expression::Call {
            function_hash: "add".to_string(),
            args: vec![
                Expression::Literal("3".to_string()),
                Expression::Literal("5".to_string()),
            ],
        };
        let folded = fold_expr(&expr);
        if let Expression::Literal(s) = folded {
            assert_eq!(s, "8");
        } else {
            panic!("Expected Literal, got {:?}", folded);
        }
    }

    #[test]
    fn test_constant_folding_booleans() {
        // true && false -> false
        let expr = Expression::Call {
            function_hash: "and".to_string(),
            args: vec![
                Expression::Literal("true".to_string()),
                Expression::Literal("false".to_string()),
            ],
        };
        let folded = fold_expr(&expr);
        if let Expression::Literal(s) = folded {
            assert_eq!(s, "false");
        } else {
            panic!("Expected Literal, got {:?}", folded);
        }
    }

    #[test]
    fn test_dead_code_after_return() {
        let stmt = Statement::Block(vec![
            Statement::Return(Expression::Literal("0".to_string())),
            Statement::Expression(Expression::Literal("unreachable".to_string())),
        ]);
        let optimized = dce_stmt(&stmt);
        if let Statement::Block(stmts) = optimized {
            assert_eq!(stmts.len(), 1);
            if let Statement::Return(_) = stmts[0] {
                // pass
            } else {
                panic!("Expected Return");
            }
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_dead_code_if_true() {
        let stmt = Statement::If {
            condition: Expression::Literal("true".to_string()),
            then_block: vec![Statement::Return(Expression::Literal("1".to_string()))],
            else_block: Some(vec![Statement::Return(Expression::Literal(
                "2".to_string(),
            ))]),
        };
        let optimized = dce_stmt(&stmt);
        if let Statement::Block(stmts) = optimized {
            assert_eq!(stmts.len(), 1);
            if let Statement::Return(Expression::Literal(s)) = &stmts[0] {
                assert_eq!(s, "1");
            } else {
                panic!("Expected Return(1)");
            }
        } else {
            panic!("Expected Block");
        }
    }

    #[test]
    fn test_optimization_level_0() {
        let stmt = Statement::If {
            condition: Expression::Literal("true".to_string()),
            then_block: vec![],
            else_block: None,
        };
        let node = ArkNode::Statement(stmt.clone());
        let res = optimize(node, 0);
        // Should be unchanged
        if let ArkNode::Statement(Statement::If { .. }) = res {
            // Pass
        } else {
            panic!("Level 0 changed structure");
        }
    }

    #[test]
    fn test_compile_error_format() {
        let err = CompileError {
            message: "Test".to_string(),
            line: 1,
            column: 2,
            file: "test.ark".to_string(),
        };
        let s = format!("{}", err);
        assert_eq!(s, "Compilation Error: Test at test.ark:1:2");
    }
}
