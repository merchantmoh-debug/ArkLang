/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * IR/AST Snapshot Tests — ensures parser and compiler output are
 * stable across changes. If a commit silently mutates the AST or
 * bytecode for any example program, these tests fail with a clear diff.
 *
 * Uses the `insta` crate for snapshot management.
 * Run `cargo insta review` to accept new snapshots after intentional changes.
 */

use crate::compiler::Compiler;
use crate::parser::parse_source;

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Parse source into an ArkNode, panicking on failure.
fn parse(source: &str, file: &str) -> crate::ast::ArkNode {
    parse_source(source, file).expect("parse failed")
}

/// Parse and compile source into bytecode opcode listing.
fn compile_opcodes(source: &str, file: &str) -> Vec<String> {
    let ast = parse(source, file);
    let chunk = Compiler::new().compile(&ast);
    chunk.code.iter().map(|op| format!("{:?}", op)).collect()
}

// ─── Example File AST Snapshots ─────────────────────────────────────────────

#[test]
fn snapshot_ast_snake() {
    let source = include_str!("../../examples/snake.ark");
    let ast = parse(source, "snake.ark");
    insta::assert_debug_snapshot!("snake_ast", ast);
}

#[test]
fn snapshot_ast_server() {
    let source = include_str!("../../examples/server.ark");
    let ast = parse(source, "server.ark");
    insta::assert_debug_snapshot!("server_ast", ast);
}

#[test]
fn snapshot_ast_simple_server() {
    let source = include_str!("../../examples/simple_server.ark");
    let ast = parse(source, "simple_server.ark");
    insta::assert_debug_snapshot!("simple_server_ast", ast);
}

// ─── Example File Bytecode Snapshots ────────────────────────────────────────

#[test]
fn snapshot_bytecode_snake() {
    let source = include_str!("../../examples/snake.ark");
    let opcodes = compile_opcodes(source, "snake.ark");
    insta::assert_debug_snapshot!("snake_bytecode", opcodes);
}

#[test]
fn snapshot_bytecode_server() {
    let source = include_str!("../../examples/server.ark");
    let opcodes = compile_opcodes(source, "server.ark");
    insta::assert_debug_snapshot!("server_bytecode", opcodes);
}

#[test]
fn snapshot_bytecode_simple_server() {
    let source = include_str!("../../examples/simple_server.ark");
    let opcodes = compile_opcodes(source, "simple_server.ark");
    insta::assert_debug_snapshot!("simple_server_bytecode", opcodes);
}

// ─── Targeted Regression Snippets ───────────────────────────────────────────
// Small, focused programs that exercise specific compiler paths.
// If a parser or codegen change breaks these, we catch it instantly.

#[test]
fn snapshot_ast_if_else() {
    let source = r#"
        x := 10
        if x > 5 {
            print("big")
        } else {
            print("small")
        }
    "#;
    let ast = parse(source, "if_else.ark");
    insta::assert_debug_snapshot!("if_else_ast", ast);
}

#[test]
fn snapshot_bytecode_if_else() {
    let source = r#"
        x := 10
        if x > 5 {
            print("big")
        } else {
            print("small")
        }
    "#;
    let opcodes = compile_opcodes(source, "if_else.ark");
    insta::assert_debug_snapshot!("if_else_bytecode", opcodes);
}

#[test]
fn snapshot_ast_while_loop() {
    let source = r#"
        i := 0
        while i < 10 {
            i := i + 1
        }
    "#;
    let ast = parse(source, "while.ark");
    insta::assert_debug_snapshot!("while_loop_ast", ast);
}

#[test]
fn snapshot_bytecode_while_loop() {
    let source = r#"
        i := 0
        while i < 10 {
            i := i + 1
        }
    "#;
    let opcodes = compile_opcodes(source, "while.ark");
    insta::assert_debug_snapshot!("while_loop_bytecode", opcodes);
}

#[test]
fn snapshot_ast_func_def() {
    let source = r#"
        func add(a, b) {
            return a + b
        }
        result := add(3, 4)
        print(result)
    "#;
    let ast = parse(source, "func.ark");
    insta::assert_debug_snapshot!("func_def_ast", ast);
}

#[test]
fn snapshot_bytecode_func_def() {
    let source = r#"
        func add(a, b) {
            return a + b
        }
        result := add(3, 4)
        print(result)
    "#;
    let opcodes = compile_opcodes(source, "func.ark");
    insta::assert_debug_snapshot!("func_def_bytecode", opcodes);
}

#[test]
fn snapshot_ast_struct_init() {
    let source = r#"
        state := {
            x: 10,
            y: 20,
            name: "test"
        }
        print(state.x)
    "#;
    let ast = parse(source, "struct.ark");
    insta::assert_debug_snapshot!("struct_init_ast", ast);
}

#[test]
fn snapshot_bytecode_struct_init() {
    let source = r#"
        state := {
            x: 10,
            y: 20,
            name: "test"
        }
        print(state.x)
    "#;
    let opcodes = compile_opcodes(source, "struct.ark");
    insta::assert_debug_snapshot!("struct_init_bytecode", opcodes);
}

#[test]
fn snapshot_ast_list_ops() {
    let source = r#"
        items := [1, 2, 3, 4, 5]
        let (l, _) := sys.len(items)
        print(l)
    "#;
    let ast = parse(source, "list.ark");
    insta::assert_debug_snapshot!("list_ops_ast", ast);
}

#[test]
fn snapshot_bytecode_list_ops() {
    let source = r#"
        items := [1, 2, 3, 4, 5]
        let (l, _) := sys.len(items)
        print(l)
    "#;
    let opcodes = compile_opcodes(source, "list.ark");
    insta::assert_debug_snapshot!("list_ops_bytecode", opcodes);
}

#[test]
fn snapshot_ast_nested_calls() {
    let source = r#"
        func outer() {
            func inner(x) {
                return x * 2
            }
            return inner(21)
        }
        print(outer())
    "#;
    let ast = parse(source, "nested.ark");
    insta::assert_debug_snapshot!("nested_calls_ast", ast);
}

#[test]
fn snapshot_bytecode_nested_calls() {
    let source = r#"
        func outer() {
            func inner(x) {
                return x * 2
            }
            return inner(21)
        }
        print(outer())
    "#;
    let opcodes = compile_opcodes(source, "nested.ark");
    insta::assert_debug_snapshot!("nested_calls_bytecode", opcodes);
}

// ─── JSON Snapshot (MAST Serialization) ─────────────────────────────────────
// Tests that the serialized JSON (the format used in snake.json etc.) is stable.

#[test]
fn snapshot_json_simple_program() {
    let source = r#"
        x := 42
        print(x)
    "#;
    let ast = parse(source, "simple.ark");
    insta::assert_json_snapshot!("simple_program_json", ast);
}
