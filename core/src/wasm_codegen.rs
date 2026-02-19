/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * WASM Native Code Generator for the Ark Language.
 *
 * Compiles ArkNode AST directly to WASM binary format via `wasm-encoder`.
 * This is a parallel backend to the bytecode compiler (compiler.rs),
 * producing native .wasm modules instead of interpreted bytecode.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

use crate::ast::{ArkNode, Expression, FunctionDef, MastNode, Pattern, Statement};
use crate::compiler::optimize;
use std::collections::HashMap;
use std::fmt;
use wasm_encoder::{
    BlockType, CodeSection, ElementSection, ExportKind, ExportSection, Function, FunctionSection,
    GlobalSection, GlobalType, ImportSection, Instruction, MemorySection, MemoryType, Module,
    TableSection, TableType, TypeSection, ValType,
};

// =============================================================================
// Error Types
// =============================================================================

#[derive(Debug, Clone)]
pub struct WasmCompileError {
    pub message: String,
    pub context: String,
}

impl fmt::Display for WasmCompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WASM compile error: {} ({})", self.message, self.context)
    }
}

impl std::error::Error for WasmCompileError {}

// =============================================================================
// WASM Value Representation
// =============================================================================

/// In WASM, Ark values are represented as i64 with a tag scheme:
///
/// - Integers: raw i64 value
/// - Booleans: 0 or 1 as i64
/// - Strings: pointer to linear memory (upper 32 bits = ptr, lower 32 = len)
/// - Unit: 0i64
///
/// For the initial backend, we focus on Integer + Boolean + Unit (i64-only),
/// which covers arithmetic, control flow, and function calls. String/List/Struct
/// support uses linear memory and will be added incrementally.
// WASI type indices (assigned in register_wasi_imports, must match push order)
const WASI_FD_WRITE_TYPE_IDX: u32 = 0; // (i32,i32,i32,i32)->i32  (fd_write + fd_read)
const WASI_CLOCK_TIME_GET_TYPE_IDX: u32 = 1; // (i32,i64,i32)->i32
const WASI_RANDOM_GET_TYPE_IDX: u32 = 2; // (i32,i32)->i32           (random_get, args_*, environ_*)
const WASI_PROC_EXIT_TYPE_IDX: u32 = 3; // (i32)->()
const WASI_PATH_OPEN_TYPE_IDX: u32 = 4; // (i32,i32,i32,i32,i32,i64,i64,i32,i32)->i32
const WASI_FD_CLOSE_TYPE_IDX: u32 = 5; // (i32)->i32

// Import function indices (0..10)
const WASI_FD_WRITE_FUNC_IDX: u32 = 0;
const WASI_FD_READ_FUNC_IDX: u32 = 1;
const WASI_CLOCK_TIME_GET_FUNC_IDX: u32 = 2;
const WASI_RANDOM_GET_FUNC_IDX: u32 = 3;
const WASI_ARGS_GET_FUNC_IDX: u32 = 4;
const WASI_ARGS_SIZES_GET_FUNC_IDX: u32 = 5;
const WASI_ENVIRON_GET_FUNC_IDX: u32 = 6;
const WASI_ENVIRON_SIZES_GET_FUNC_IDX: u32 = 7;
const WASI_PROC_EXIT_FUNC_IDX: u32 = 8;
const WASI_PATH_OPEN_FUNC_IDX: u32 = 9;
const WASI_FD_CLOSE_FUNC_IDX: u32 = 10;
const WASI_IMPORT_COUNT: u32 = 11;

// Ark Host type indices (extend after WASI types, pushed in register_wasi_imports)
const ARK_HOST_UNARY_I64_TYPE_IDX: u32 = 6; // (i64) -> i64  (math unary)
const ARK_HOST_BINARY_I64_TYPE_IDX: u32 = 7; // (i64,i64) -> i64  (math binary)
const ARK_HOST_TERNARY_I64_TYPE_IDX: u32 = 8; // (i64,i64,i64) -> i64  (pow_mod)
const ARK_HOST_MEM_3I32_TYPE_IDX: u32 = 9; // (i32,i32,i32) -> i32  (crypto_sha512, json_parse, json_stringify)
const ARK_HOST_MEM_4I32_TYPE_IDX: u32 = 10; // (i32,i32,i32,i32) -> i32  (ask_ai)

// Ark Host function indices (offset from WASI imports)
const ARK_HOST_MATH_SIN_FUNC_IDX: u32 = 11;
const ARK_HOST_MATH_COS_FUNC_IDX: u32 = 12;
const ARK_HOST_MATH_TAN_FUNC_IDX: u32 = 13;
const ARK_HOST_MATH_ASIN_FUNC_IDX: u32 = 14;
const ARK_HOST_MATH_ACOS_FUNC_IDX: u32 = 15;
const ARK_HOST_MATH_ATAN_FUNC_IDX: u32 = 16;
const ARK_HOST_MATH_ATAN2_FUNC_IDX: u32 = 17;
const ARK_HOST_MATH_SQRT_FUNC_IDX: u32 = 18;
const ARK_HOST_MATH_POW_FUNC_IDX: u32 = 19;
const ARK_HOST_MATH_POW_MOD_FUNC_IDX: u32 = 20;
const ARK_HOST_CRYPTO_SHA512_FUNC_IDX: u32 = 21;
const ARK_HOST_JSON_PARSE_FUNC_IDX: u32 = 22;
const ARK_HOST_JSON_STRINGIFY_FUNC_IDX: u32 = 23;
const ARK_HOST_ASK_AI_FUNC_IDX: u32 = 24;

const ARK_HOST_IMPORT_COUNT: u32 = 14;
const TOTAL_IMPORT_COUNT: u32 = WASI_IMPORT_COUNT + ARK_HOST_IMPORT_COUNT; // 25

const STRING_MEMORY_START: i32 = 1024; // strings start at byte 1024 in linear memory

// =============================================================================
// Scope & Local Variable Tracking
// =============================================================================

/// Tracks local variable name → WASM local index mapping within a function.
#[derive(Debug, Clone)]
struct LocalScope {
    /// Map of variable name → local index
    locals: HashMap<String, u32>,
    /// Next available local index
    next_local: u32,
}

impl LocalScope {
    fn new(param_count: u32) -> Self {
        Self {
            locals: HashMap::new(),
            next_local: param_count,
        }
    }

    /// Get or allocate a local variable index for the given name.
    fn get_or_alloc(&mut self, name: &str) -> u32 {
        if let Some(&idx) = self.locals.get(name) {
            idx
        } else {
            let idx = self.next_local;
            self.locals.insert(name.to_string(), idx);
            self.next_local += 1;
            idx
        }
    }

    /// Get a local variable index (returns None if not found).
    fn get(&self, name: &str) -> Option<u32> {
        self.locals.get(name).copied()
    }

    /// Total number of extra locals (beyond parameters).
    fn extra_local_count(&self, param_count: u32) -> u32 {
        if self.next_local > param_count {
            self.next_local - param_count
        } else {
            0
        }
    }
}

// =============================================================================
// Function Compilation Context
// =============================================================================

/// Context for compiling a single function body.
struct FuncContext {
    scope: LocalScope,
    instructions: Vec<Instruction<'static>>,
    param_count: u32,
    /// String data to be placed in linear memory (collected during compilation)
    string_data: Vec<(i32, Vec<u8>)>, // (offset, bytes)
    /// Next available string memory offset
    string_offset: i32,
}

impl FuncContext {
    fn new(param_count: u32) -> Self {
        Self {
            scope: LocalScope::new(param_count),
            instructions: Vec::new(),
            param_count,
            string_data: Vec::new(),
            string_offset: STRING_MEMORY_START,
        }
    }

    fn emit(&mut self, instr: Instruction<'static>) {
        self.instructions.push(instr);
    }

    /// Allocate a string in linear memory, return (ptr, len).
    fn alloc_string(&mut self, s: &str) -> (i32, i32) {
        let bytes = s.as_bytes();
        let ptr = self.string_offset;
        let len = bytes.len() as i32;
        self.string_data.push((ptr, bytes.to_vec()));
        self.string_offset += len;
        // Align to 8 bytes
        self.string_offset = (self.string_offset + 7) & !7;
        (ptr, len)
    }
}

// =============================================================================
// WASM Code Generator
// =============================================================================

/// The main WASM code generator. Compiles an ArkNode AST into a WASM binary.
pub struct WasmCodegen {
    /// Type section: function signatures
    types: Vec<(Vec<ValType>, Vec<ValType>)>,
    /// Functions: (type_index, name, func_context)
    functions: Vec<(u32, String, FuncContext)>,
    /// WASI import count (shifts function indices)
    import_count: u32,
    /// String constants to embed in data section
    data_segments: Vec<(i32, Vec<u8>)>,
    /// Next data segment offset
    #[allow(dead_code)]
    data_offset: i32,
    /// Global function name → function index
    func_index_map: HashMap<String, u32>,
    /// Starting offset of the heap region (after string data)
    heap_start: i32,
    /// Per-function attributes (name → Vec<"export", "golem::handler", etc.>)
    func_attributes: HashMap<String, Vec<String>>,
    /// Counter for generating unique lambda names
    lambda_counter: u32,
}

impl Default for WasmCodegen {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmCodegen {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            functions: Vec::new(),
            import_count: 0,
            data_segments: Vec::new(),
            data_offset: 0,
            func_index_map: HashMap::new(),
            heap_start: STRING_MEMORY_START, // will be bumped during compilation
            func_attributes: HashMap::new(),
            lambda_counter: 0,
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /// Compile an ArkNode AST to a WASM binary (Vec<u8>).
    /// This is the main entry point — equivalent to `Compiler::compile()`.
    pub fn compile(mut self, node: &ArkNode) -> Result<Vec<u8>, WasmCompileError> {
        // Phase 1: Optimize the AST (reuse existing optimizer)
        let optimized = optimize(node.clone(), 2);

        // Phase 2: Register WASI imports
        self.register_wasi_imports();

        // Phase 3: Collect all top-level function definitions first (forward declarations)
        self.collect_functions(&optimized)?;

        // Phase 3.5: Collect and register lambdas as synthetic top-level functions
        self.collect_lambdas(&optimized)?;

        // Phase 4: Compile each function body
        self.compile_collected_functions(&optimized)?;

        // Phase 4.5: Compile lambda function bodies
        self.compile_lambda_bodies(&optimized)?;

        // Phase 5: Compile top-level code as `_start` function
        self.compile_start_function(&optimized)?;

        // Phase 6: Emit the WASM module
        Ok(self.emit_module())
    }

    /// Compile to WASM and return only the raw bytes (convenience).
    pub fn compile_to_bytes(node: &ArkNode) -> Result<Vec<u8>, WasmCompileError> {
        let codegen = Self::new();
        codegen.compile(node)
    }

    // =========================================================================
    // WASI Import Registration
    // =========================================================================

    fn register_wasi_imports(&mut self) {
        // =====================================================================
        // WASI Preview1 type signatures (6 unique types for 11 imports)
        // =====================================================================

        // Type 0: fd_write / fd_read (fd:i32, iovs:i32, iovs_len:i32, nwritten:i32) -> i32
        self.types.push((
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        ));

        // Type 1: clock_time_get(clock_id:i32, precision:i64, timestamp_ptr:i32) -> i32
        self.types.push((
            vec![ValType::I32, ValType::I64, ValType::I32],
            vec![ValType::I32],
        ));

        // Type 2: random_get(buf:i32, buf_len:i32) -> i32
        self.types
            .push((vec![ValType::I32, ValType::I32], vec![ValType::I32]));

        // Type 3: args_get / args_sizes_get / environ_get / environ_sizes_get / fd_close
        //         (ptr1:i32, ptr2:i32) -> i32
        // (same signature as random_get but semantically different — reuse type 2)
        // Actually fd_close is (fd:i32) -> i32, needs its own. Let's keep type 3 = (i32,i32)->i32
        // We can reuse type 2 for these since signature matches.

        // Type 3: proc_exit(code:i32) -> ()
        self.types.push((vec![ValType::I32], vec![]));

        // Type 4: path_open(fd:i32, dirflags:i32, path:i32, path_len:i32,
        //                   oflags:i32, rights_base:i64, rights_inherit:i64,
        //                   fdflags:i32, opened_fd:i32) -> i32
        self.types.push((
            vec![
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I64,
                ValType::I64,
                ValType::I32,
                ValType::I32,
            ],
            vec![ValType::I32],
        ));

        // Type 5: fd_close(fd:i32) -> i32
        self.types.push((vec![ValType::I32], vec![ValType::I32]));

        // =====================================================================
        // Ark Host Import type signatures (5 unique types for 14 imports)
        // =====================================================================

        // Type 6: (i64) -> i64  — math unary (sin, cos, tan, asin, acos, atan, sqrt)
        self.types.push((vec![ValType::I64], vec![ValType::I64]));

        // Type 7: (i64, i64) -> i64  — math binary (atan2, pow)
        self.types
            .push((vec![ValType::I64, ValType::I64], vec![ValType::I64]));

        // Type 8: (i64, i64, i64) -> i64  — math ternary (pow_mod)
        self.types.push((
            vec![ValType::I64, ValType::I64, ValType::I64],
            vec![ValType::I64],
        ));

        // Type 9: (i32, i32, i32) -> i32  — memory-based (crypto_sha512, json_parse, json_stringify)
        self.types.push((
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        ));

        // Type 10: (i32, i32, i32, i32) -> i32  — memory-based (ask_ai)
        // Note: same as WASI fd_write type but under ark_host module
        self.types.push((
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        ));

        self.import_count = TOTAL_IMPORT_COUNT;

        // =====================================================================
        // __alloc internal function
        // =====================================================================

        // Type for __alloc: (size: i64) -> i64 (matches Ark's i64 calling convention)
        let alloc_type_idx = self.types.len() as u32;
        self.types.push((vec![ValType::I64], vec![ValType::I64]));
        let alloc_func_idx = self.import_count + self.functions.len() as u32;
        self.func_index_map
            .insert("__alloc".to_string(), alloc_func_idx);

        // Build __alloc body:
        //   fn __alloc(size: i32) -> i32 {
        //       let ptr = global.__heap_ptr;
        //       global.__heap_ptr = ptr + ((size + 7) & !7);  // 8-byte aligned
        //       return ptr;
        //   }
        let mut ctx = FuncContext::new(1); // 1 parameter: size
        let ptr_local = ctx.scope.get_or_alloc("__alloc_ptr");

        // ptr = global.get(0)  (__heap_ptr is global index 0)
        ctx.emit(Instruction::GlobalGet(0));
        ctx.emit(Instruction::I64ExtendI32U); // convert to i64 to store in local
        ctx.emit(Instruction::LocalSet(ptr_local));

        // global.set(0, ptr + align8(size))
        // align8(size) = (size + 7) & ~7
        ctx.emit(Instruction::LocalGet(ptr_local));
        ctx.emit(Instruction::I32WrapI64); // ptr as i32
        ctx.emit(Instruction::LocalGet(0)); // size param (i64)
        ctx.emit(Instruction::I32WrapI64); // size as i32
        ctx.emit(Instruction::I32Const(7));
        ctx.emit(Instruction::I32Add); // size + 7
        ctx.emit(Instruction::I32Const(!7)); // ~7 = 0xFFFF_FFF8
        ctx.emit(Instruction::I32And); // (size + 7) & ~7
        ctx.emit(Instruction::I32Add); // ptr + aligned_size
        ctx.emit(Instruction::GlobalSet(0)); // update __heap_ptr

        // return ptr (already i64 from the I64ExtendI32U at store time)
        ctx.emit(Instruction::LocalGet(ptr_local));
        ctx.emit(Instruction::End);

        self.functions
            .push((alloc_type_idx, "__alloc".to_string(), ctx));
    }

    // =========================================================================
    // Function Collection (Forward Declarations)
    // =========================================================================

    fn collect_functions(&mut self, node: &ArkNode) -> Result<(), WasmCompileError> {
        match node {
            ArkNode::Function(func_def) => {
                self.register_function(func_def)?;
            }
            ArkNode::Statement(Statement::Function(func_def)) => {
                self.register_function(func_def)?;
            }
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    if let Statement::Function(func_def) = stmt {
                        self.register_function(func_def)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    // =========================================================================
    // Lambda Collection (Pre-scan for lambda lifting)
    // =========================================================================

    /// Walk the AST to find all `Expression::Lambda` nodes and register them
    /// as synthetic top-level functions (`__lambda_0`, `__lambda_1`, etc.).
    /// This enables call_indirect dispatch for higher-order functions.
    fn collect_lambdas(&mut self, node: &ArkNode) -> Result<(), WasmCompileError> {
        match node {
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    self.scan_stmt_for_lambdas(stmt)?;
                }
            }
            ArkNode::Statement(stmt) => {
                self.scan_stmt_for_lambdas(stmt)?;
            }
            ArkNode::Function(func_def) => {
                self.scan_stmts_for_lambdas_in_body(&func_def.body)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Recursively scan a statement for lambda expressions.
    fn scan_stmt_for_lambdas(&mut self, stmt: &Statement) -> Result<(), WasmCompileError> {
        match stmt {
            Statement::Let { value, .. } => {
                self.scan_expr_for_lambdas(value)?;
            }
            Statement::Block(stmts) => {
                for s in stmts {
                    self.scan_stmt_for_lambdas(s)?;
                }
            }
            Statement::Expression(expr) => {
                self.scan_expr_for_lambdas(expr)?;
            }
            Statement::Return(expr) => {
                self.scan_expr_for_lambdas(expr)?;
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.scan_expr_for_lambdas(condition)?;
                for s in then_block {
                    self.scan_stmt_for_lambdas(s)?;
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        self.scan_stmt_for_lambdas(s)?;
                    }
                }
            }
            Statement::While { condition, body } => {
                self.scan_expr_for_lambdas(condition)?;
                for s in body {
                    self.scan_stmt_for_lambdas(s)?;
                }
            }
            Statement::Function(func_def) => {
                self.scan_stmts_for_lambdas_in_body(&func_def.body)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Scan a MastNode body for lambdas.
    fn scan_stmts_for_lambdas_in_body(&mut self, body: &MastNode) -> Result<(), WasmCompileError> {
        match &body.content {
            ArkNode::Statement(Statement::Block(stmts)) => {
                for s in stmts {
                    self.scan_stmt_for_lambdas(s)?;
                }
            }
            ArkNode::Statement(stmt) => {
                self.scan_stmt_for_lambdas(stmt)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Scan an expression for lambda sub-expressions and register them.
    fn scan_expr_for_lambdas(&mut self, expr: &Expression) -> Result<(), WasmCompileError> {
        match expr {
            Expression::Lambda {
                params,
                body: _body,
            } => {
                // Register this lambda as a synthetic top-level function
                let lambda_name = format!("__lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                let param_types: Vec<ValType> = params.iter().map(|_| ValType::I64).collect();
                let return_types = vec![ValType::I64];

                let type_idx = self.types.len() as u32;
                self.types.push((param_types, return_types));

                let func_idx = self.import_count + self.functions.len() as u32;
                self.func_index_map.insert(lambda_name.clone(), func_idx);

                let ctx = FuncContext::new(params.len() as u32);
                self.functions.push((type_idx, lambda_name, ctx));
            }
            Expression::Call { args, .. } => {
                for arg in args {
                    self.scan_expr_for_lambdas(arg)?;
                }
            }
            Expression::List(items) => {
                for item in items {
                    self.scan_expr_for_lambdas(item)?;
                }
            }
            Expression::StructInit { fields } => {
                for (_, val) in fields {
                    self.scan_expr_for_lambdas(val)?;
                }
            }
            Expression::GetField { obj, .. } => {
                self.scan_expr_for_lambdas(obj)?;
            }
            Expression::Match { scrutinee, arms } => {
                self.scan_expr_for_lambdas(scrutinee)?;
                for (_, arm_expr) in arms {
                    self.scan_expr_for_lambdas(arm_expr)?;
                }
            }
            _ => {} // Variable, Literal, Integer — no lambdas inside
        }
        Ok(())
    }

    // =========================================================================
    // Lambda Body Compilation
    // =========================================================================

    /// Compile the bodies of all registered lambda functions.
    /// Walks the AST again to find Expression::Lambda nodes, matches them
    /// to their registered __lambda_N slots, and compiles the body.
    fn compile_lambda_bodies(&mut self, node: &ArkNode) -> Result<(), WasmCompileError> {
        let mut lambda_exprs: Vec<(Vec<String>, Vec<Statement>)> = Vec::new();
        Self::collect_lambda_exprs(node, &mut lambda_exprs);

        let func_index_map = self.func_index_map.clone();

        for (idx, (params, body)) in lambda_exprs.iter().enumerate() {
            let lambda_name = format!("__lambda_{}", idx);
            if let Some(&func_idx) = func_index_map.get(&lambda_name) {
                // Find which slot in self.functions corresponds to this func_idx
                let slot = (func_idx - self.import_count) as usize;
                if slot < self.functions.len() {
                    let param_count = params.len() as u32;
                    let mut ctx = FuncContext::new(param_count);

                    // Register parameter names as locals
                    for (j, name) in params.iter().enumerate() {
                        ctx.scope.locals.insert(name.clone(), j as u32);
                    }

                    // Compile each body statement
                    let body_len = body.len();
                    if body_len == 0 {
                        ctx.emit(Instruction::I64Const(0));
                    } else {
                        for (i, stmt) in body.iter().enumerate() {
                            let is_last = i == body_len - 1;
                            Self::compile_stmt(&mut ctx, stmt, is_last, &func_index_map)?;
                        }
                    }

                    // Ensure End opcode
                    ctx.emit(Instruction::End);

                    // Place compiled context in the right function slot
                    self.functions[slot].2 = ctx;
                }
            }
        }

        Ok(())
    }

    /// Collect all lambda expressions from the AST (in order of appearance).
    fn collect_lambda_exprs(node: &ArkNode, out: &mut Vec<(Vec<String>, Vec<Statement>)>) {
        match node {
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    Self::collect_lambda_exprs_from_stmt(stmt, out);
                }
            }
            ArkNode::Statement(stmt) => {
                Self::collect_lambda_exprs_from_stmt(stmt, out);
            }
            ArkNode::Function(func_def) => {
                Self::collect_lambda_exprs_from_mast(&func_def.body, out);
            }
            _ => {}
        }
    }

    fn collect_lambda_exprs_from_stmt(
        stmt: &Statement,
        out: &mut Vec<(Vec<String>, Vec<Statement>)>,
    ) {
        match stmt {
            Statement::Let { value, .. } => {
                Self::collect_lambda_exprs_from_expr(value, out);
            }
            Statement::Block(stmts) => {
                for s in stmts {
                    Self::collect_lambda_exprs_from_stmt(s, out);
                }
            }
            Statement::Expression(expr) => {
                Self::collect_lambda_exprs_from_expr(expr, out);
            }
            Statement::Return(expr) => {
                Self::collect_lambda_exprs_from_expr(expr, out);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::collect_lambda_exprs_from_expr(condition, out);
                for s in then_block {
                    Self::collect_lambda_exprs_from_stmt(s, out);
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        Self::collect_lambda_exprs_from_stmt(s, out);
                    }
                }
            }
            Statement::While { condition, body } => {
                Self::collect_lambda_exprs_from_expr(condition, out);
                for s in body {
                    Self::collect_lambda_exprs_from_stmt(s, out);
                }
            }
            Statement::Function(func_def) => {
                Self::collect_lambda_exprs_from_mast(&func_def.body, out);
            }
            _ => {}
        }
    }

    fn collect_lambda_exprs_from_mast(
        mast: &MastNode,
        out: &mut Vec<(Vec<String>, Vec<Statement>)>,
    ) {
        match &mast.content {
            ArkNode::Statement(Statement::Block(stmts)) => {
                for s in stmts {
                    Self::collect_lambda_exprs_from_stmt(s, out);
                }
            }
            ArkNode::Statement(stmt) => {
                Self::collect_lambda_exprs_from_stmt(stmt, out);
            }
            _ => {}
        }
    }

    fn collect_lambda_exprs_from_expr(
        expr: &Expression,
        out: &mut Vec<(Vec<String>, Vec<Statement>)>,
    ) {
        match expr {
            Expression::Lambda { params, body } => {
                out.push((params.clone(), body.clone()));
            }
            Expression::Call { args, .. } => {
                for arg in args {
                    Self::collect_lambda_exprs_from_expr(arg, out);
                }
            }
            Expression::List(items) => {
                for item in items {
                    Self::collect_lambda_exprs_from_expr(item, out);
                }
            }
            Expression::StructInit { fields } => {
                for (_, val) in fields {
                    Self::collect_lambda_exprs_from_expr(val, out);
                }
            }
            Expression::GetField { obj, .. } => {
                Self::collect_lambda_exprs_from_expr(obj, out);
            }
            Expression::Match { scrutinee, arms } => {
                Self::collect_lambda_exprs_from_expr(scrutinee, out);
                for (_, arm_expr) in arms {
                    Self::collect_lambda_exprs_from_expr(arm_expr, out);
                }
            }
            _ => {}
        }
    }

    fn register_function(&mut self, func_def: &FunctionDef) -> Result<(), WasmCompileError> {
        let param_types: Vec<ValType> = func_def.inputs.iter().map(|_| ValType::I64).collect();
        let return_types = vec![ValType::I64]; // All functions return i64 for now

        let type_idx = self.types.len() as u32;
        self.types.push((param_types, return_types));

        let func_idx = self.import_count + self.functions.len() as u32;
        self.func_index_map.insert(func_def.name.clone(), func_idx);

        // Create empty context — will be filled during compile phase
        let ctx = FuncContext::new(func_def.inputs.len() as u32);
        self.functions.push((type_idx, func_def.name.clone(), ctx));

        // Store function attributes for selective export decisions
        if !func_def.attributes.is_empty() {
            self.func_attributes
                .insert(func_def.name.clone(), func_def.attributes.clone());
        }

        Ok(())
    }

    // =========================================================================
    // Selective Export Logic
    // =========================================================================

    /// Determine if a function should be exported in the WASM binary.
    /// Returns true if the function has the `#[export]` attribute
    /// or is a system function (_start, main, __alloc).
    fn should_export(&self, name: &str) -> bool {
        // System functions are always exported
        if matches!(name, "_start" | "main" | "__alloc") {
            return true;
        }
        // Check if function has #[export] attribute
        if let Some(attrs) = self.func_attributes.get(name) {
            return attrs.iter().any(|a| a == "export");
        }
        false
    }

    // =========================================================================
    // Function Body Compilation
    // =========================================================================

    fn compile_collected_functions(&mut self, node: &ArkNode) -> Result<(), WasmCompileError> {
        // We need to build contexts for each registered function
        let func_defs = self.extract_function_defs(node);
        let func_index_map = self.func_index_map.clone();

        for (i, func_def) in func_defs.iter().enumerate() {
            let param_count = func_def.inputs.len() as u32;
            let mut ctx = FuncContext::new(param_count);

            // Register parameter names as locals
            for (j, (name, _)) in func_def.inputs.iter().enumerate() {
                ctx.scope.locals.insert(name.clone(), j as u32);
            }

            // Compile the function body
            Self::compile_mast_node(&mut ctx, &func_def.body, &func_index_map)?;

            // Ensure we return something
            ctx.emit(Instruction::End);

            // Skip internal functions (__alloc is at index 0)
            let internal_offset = 1; // __alloc
            self.functions[i + internal_offset].2 = ctx;
        }

        Ok(())
    }

    fn extract_function_defs(&self, node: &ArkNode) -> Vec<FunctionDef> {
        let mut defs = Vec::new();
        match node {
            ArkNode::Function(func_def) => {
                defs.push(func_def.clone());
            }
            ArkNode::Statement(Statement::Function(func_def)) => {
                defs.push(func_def.clone());
            }
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    if let Statement::Function(func_def) = stmt {
                        defs.push(func_def.clone());
                    }
                }
            }
            _ => {}
        }
        defs
    }

    // =========================================================================
    // _start Function (Top-Level Code)
    // =========================================================================

    fn compile_start_function(&mut self, node: &ArkNode) -> Result<(), WasmCompileError> {
        let type_idx = self.types.len() as u32;
        // _start: () -> ()
        self.types.push((vec![], vec![]));

        let func_idx = self.import_count + self.functions.len() as u32;
        self.func_index_map.insert("_start".to_string(), func_idx);

        let mut ctx = FuncContext::new(0);
        let func_index_map = self.func_index_map.clone();

        match node {
            ArkNode::Statement(Statement::Block(stmts)) => {
                for stmt in stmts {
                    // Skip function definitions (already compiled)
                    if matches!(stmt, Statement::Function(_)) {
                        continue;
                    }
                    Self::compile_stmt(&mut ctx, stmt, false, &func_index_map)?;
                }
            }
            ArkNode::Statement(stmt) => {
                if !matches!(stmt, Statement::Function(_)) {
                    Self::compile_stmt(&mut ctx, stmt, false, &func_index_map)?;
                }
            }
            ArkNode::Expression(expr) => {
                Self::compile_expr(&mut ctx, expr, &func_index_map)?;
                ctx.emit(Instruction::Drop);
            }
            _ => {}
        }

        ctx.emit(Instruction::End);
        self.functions.push((type_idx, "_start".to_string(), ctx));

        Ok(())
    }

    // =========================================================================
    // Statement Compilation
    // =========================================================================

    fn compile_mast_node(
        ctx: &mut FuncContext,
        mast: &MastNode,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        match &mast.content {
            ArkNode::Statement(stmt) => Self::compile_stmt(ctx, stmt, true, func_map),
            ArkNode::Expression(expr) => Self::compile_expr(ctx, expr, func_map),
            ArkNode::Function(_) => Ok(()), // Already handled
            ArkNode::Type(_) => Ok(()),
        }
    }

    fn compile_stmt(
        ctx: &mut FuncContext,
        stmt: &Statement,
        preserve: bool,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        match stmt {
            // -----------------------------------------------------------------
            // Let binding: evaluate expression, store in local
            // -----------------------------------------------------------------
            Statement::Let { name, value, .. } => {
                Self::compile_expr(ctx, value, func_map)?;
                let idx = ctx.scope.get_or_alloc(name);
                ctx.emit(Instruction::LocalSet(idx));
                Ok(())
            }

            // -----------------------------------------------------------------
            // Expression statement: compile and optionally drop result
            // -----------------------------------------------------------------
            Statement::Expression(expr) => {
                Self::compile_expr(ctx, expr, func_map)?;
                if !preserve {
                    ctx.emit(Instruction::Drop);
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // Block: compile each statement, last one preserves value
            // -----------------------------------------------------------------
            Statement::Block(stmts) => {
                let len = stmts.len();
                if len == 0 && preserve {
                    ctx.emit(Instruction::I64Const(0)); // Unit
                    return Ok(());
                }
                for (i, s) in stmts.iter().enumerate() {
                    let is_last = i == len - 1;
                    Self::compile_stmt(ctx, s, preserve && is_last, func_map)?;
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // Return: compile expression and return
            // -----------------------------------------------------------------
            Statement::Return(expr) => {
                Self::compile_expr(ctx, expr, func_map)?;
                ctx.emit(Instruction::Return);
                Ok(())
            }

            // -----------------------------------------------------------------
            // If/Else: uses WASM block/if structured control flow
            // -----------------------------------------------------------------
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                Self::compile_expr(ctx, condition, func_map)?;
                // Convert i64 condition to i32 for br_if
                ctx.emit(Instruction::I32WrapI64);

                if preserve {
                    // If that produces a value
                    ctx.emit(Instruction::If(wasm_encoder::BlockType::Result(
                        ValType::I64,
                    )));
                } else {
                    ctx.emit(Instruction::If(wasm_encoder::BlockType::Empty));
                }

                // Then block
                let then_len = then_block.len();
                if then_len == 0 && preserve {
                    ctx.emit(Instruction::I64Const(0)); // Unit
                } else {
                    for (i, s) in then_block.iter().enumerate() {
                        let is_last = i == then_len - 1;
                        Self::compile_stmt(ctx, s, preserve && is_last, func_map)?;
                    }
                }

                // Else block
                if let Some(else_stmts) = else_block {
                    ctx.emit(Instruction::Else);
                    let else_len = else_stmts.len();
                    if else_len == 0 && preserve {
                        ctx.emit(Instruction::I64Const(0)); // Unit
                    } else {
                        for (i, s) in else_stmts.iter().enumerate() {
                            let is_last = i == else_len - 1;
                            Self::compile_stmt(ctx, s, preserve && is_last, func_map)?;
                        }
                    }
                } else if preserve {
                    ctx.emit(Instruction::Else);
                    ctx.emit(Instruction::I64Const(0)); // implicit Unit
                }

                ctx.emit(Instruction::End);
                Ok(())
            }

            // -----------------------------------------------------------------
            // While loop: block + loop + br_if
            // -----------------------------------------------------------------
            Statement::While { condition, body } => {
                // block $break
                //   loop $continue
                //     condition
                //     i32.eqz
                //     br_if $break
                //     body...
                //     br $continue
                //   end
                // end
                ctx.emit(Instruction::Block(wasm_encoder::BlockType::Empty));
                ctx.emit(Instruction::Loop(wasm_encoder::BlockType::Empty));

                // Evaluate condition
                Self::compile_expr(ctx, condition, func_map)?;
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Eqz);
                ctx.emit(Instruction::BrIf(1)); // break out of block

                // Body
                for s in body {
                    Self::compile_stmt(ctx, s, false, func_map)?;
                }

                ctx.emit(Instruction::Br(0)); // continue loop
                ctx.emit(Instruction::End); // end loop
                ctx.emit(Instruction::End); // end block

                if preserve {
                    ctx.emit(Instruction::I64Const(0)); // Unit
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // For loop: desugar to while (not yet fully supported)
            // -----------------------------------------------------------------
            Statement::For { .. } => {
                // TODO: Implement for loop desugaring
                if preserve {
                    ctx.emit(Instruction::I64Const(0));
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // SetField: obj.field = value
            // -----------------------------------------------------------------
            Statement::SetField {
                obj_name,
                field: _field,
                value,
            } => {
                // Look up the struct pointer from the local variable
                if let Some(local_idx) = ctx.scope.locals.get(obj_name).copied() {
                    // Compile value
                    Self::compile_expr(ctx, value, func_map)?;
                    let val_local = ctx.scope.get_or_alloc("__setfield_val");
                    ctx.emit(Instruction::LocalSet(val_local));

                    // Load struct ptr
                    ctx.emit(Instruction::LocalGet(local_idx));
                    ctx.emit(Instruction::I32WrapI64);
                    // Store value at first field offset (ptr + 8)
                    // TODO: Use field name to determine offset when type info available
                    ctx.emit(Instruction::LocalGet(val_local));
                    ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                        offset: 8,
                        align: 3,
                        memory_index: 0,
                    }));
                }
                if preserve {
                    ctx.emit(Instruction::I64Const(0));
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // LetDestructure: bind names to sequential list elements
            // let [a, b, c] = my_list;
            // → a = list[0], b = list[1], c = list[2]
            // -----------------------------------------------------------------
            Statement::LetDestructure { names, value } => {
                // Compile the list/value expression
                Self::compile_expr(ctx, value, func_map)?;
                let list_ptr = ctx.scope.get_or_alloc("__destructure_ptr");
                ctx.emit(Instruction::LocalSet(list_ptr));

                // Bind each name to list element at ptr + 8 + 8*i
                for (i, name) in names.iter().enumerate() {
                    let name_local = ctx.scope.get_or_alloc(name);
                    ctx.emit(Instruction::LocalGet(list_ptr));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                        offset: (8 + 8 * i) as u64,
                        align: 3,
                        memory_index: 0,
                    }));
                    ctx.emit(Instruction::LocalSet(name_local));
                }
                if preserve {
                    ctx.emit(Instruction::I64Const(0));
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // Function: already compiled in collect phase — skip
            // -----------------------------------------------------------------
            Statement::Function(_) => Ok(()),

            // -----------------------------------------------------------------
            // Import / StructDecl: metadata only, no codegen
            // -----------------------------------------------------------------
            Statement::Import(_)
            | Statement::StructDecl(_)
            | Statement::EnumDecl(_)
            | Statement::TraitDecl(_)
            | Statement::ImplBlock(_) => Ok(()),

            // -----------------------------------------------------------------
            // Break / Continue: br instructions
            // -----------------------------------------------------------------
            Statement::Break => {
                ctx.emit(Instruction::Br(1)); // break out of enclosing block
                Ok(())
            }
            Statement::Continue => {
                ctx.emit(Instruction::Br(0)); // jump to loop header
                Ok(())
            }
        }
    }

    // =========================================================================
    // Expression Compilation
    // =========================================================================

    fn compile_expr(
        ctx: &mut FuncContext,
        expr: &Expression,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        match expr {
            // -----------------------------------------------------------------
            // Integer literal → i64.const
            // -----------------------------------------------------------------
            Expression::Integer(n) => {
                ctx.emit(Instruction::I64Const(*n));
                Ok(())
            }

            // -----------------------------------------------------------------
            // String literal → store in linear memory, push packed ptr|len
            // -----------------------------------------------------------------
            Expression::Literal(s) => {
                let (ptr, len) = ctx.alloc_string(s);
                // Pack as (ptr << 32) | len — both fit in i64
                let packed = ((ptr as i64) << 32) | (len as i64 & 0xFFFFFFFF);
                ctx.emit(Instruction::I64Const(packed));
                Ok(())
            }

            // -----------------------------------------------------------------
            // Variable → local.get
            // -----------------------------------------------------------------
            Expression::Variable(name) => {
                if let Some(idx) = ctx.scope.get(name) {
                    ctx.emit(Instruction::LocalGet(idx));
                } else {
                    // Could be a function reference — push function index as i64
                    if let Some(&func_idx) = func_map.get(name) {
                        ctx.emit(Instruction::I64Const(func_idx as i64));
                    } else {
                        // Allocate as new local (might be used before definition in some patterns)
                        let idx = ctx.scope.get_or_alloc(name);
                        ctx.emit(Instruction::LocalGet(idx));
                    }
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // Function call
            // -----------------------------------------------------------------
            Expression::Call {
                function_hash,
                args,
            } => {
                match function_hash.as_str() {
                    // Arithmetic intrinsics → direct WASM opcodes
                    "intrinsic_add" | "add" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64Add, func_map)?;
                    }
                    "intrinsic_sub" | "sub" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64Sub, func_map)?;
                    }
                    "intrinsic_mul" | "mul" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64Mul, func_map)?;
                    }
                    "intrinsic_div" | "div" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64DivS, func_map)?;
                    }
                    "intrinsic_mod" | "modulo" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64RemS, func_map)?;
                    }

                    // Comparison intrinsics → WASM comparison + extend to i64
                    "intrinsic_eq" | "eq" => {
                        Self::compile_compare_op(ctx, args, Instruction::I64Eq, func_map)?;
                    }
                    "intrinsic_neq" | "neq" => {
                        Self::compile_compare_op(ctx, args, Instruction::I64Ne, func_map)?;
                    }
                    "intrinsic_gt" | "gt" => {
                        Self::compile_compare_op(ctx, args, Instruction::I64GtS, func_map)?;
                    }
                    "intrinsic_lt" | "lt" => {
                        Self::compile_compare_op(ctx, args, Instruction::I64LtS, func_map)?;
                    }
                    "intrinsic_ge" | "ge" => {
                        Self::compile_compare_op(ctx, args, Instruction::I64GeS, func_map)?;
                    }
                    "intrinsic_le" | "le" => {
                        Self::compile_compare_op(ctx, args, Instruction::I64LeS, func_map)?;
                    }

                    // Logical intrinsics
                    "intrinsic_and" | "and" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64And, func_map)?;
                    }
                    "intrinsic_or" | "or" => {
                        Self::compile_binary_op(ctx, args, Instruction::I64Or, func_map)?;
                    }

                    // Print → WASI fd_write
                    "print" | "intrinsic_print" => {
                        Self::compile_print(ctx, args, func_map)?;
                    }

                    // =========================================================
                    // String Intrinsics (packed i64: ptr<<32 | len)
                    // =========================================================

                    // string_len(s) → extract len from packed i64
                    "string_len" | "intrinsic_string_len" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "string_len requires 1 argument".to_string(),
                                context: "string_len".to_string(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        // len = packed & 0xFFFFFFFF
                        ctx.emit(Instruction::I64Const(0xFFFFFFFF));
                        ctx.emit(Instruction::I64And);
                    }

                    // string_concat(a, b) → allocate new buffer, copy both, repack
                    "string_concat" | "intrinsic_string_concat" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "string_concat requires 2 arguments".to_string(),
                                context: "string_concat".to_string(),
                            });
                        }
                        let packed_a = ctx.scope.get_or_alloc("__str_concat_a");
                        let packed_b = ctx.scope.get_or_alloc("__str_concat_b");
                        let ptr_a = ctx.scope.get_or_alloc("__str_ptr_a");
                        let len_a = ctx.scope.get_or_alloc("__str_len_a");
                        let ptr_b = ctx.scope.get_or_alloc("__str_ptr_b");
                        let len_b = ctx.scope.get_or_alloc("__str_len_b");
                        let new_ptr = ctx.scope.get_or_alloc("__str_new_ptr");

                        // Evaluate and store both args
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(packed_a));
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::LocalSet(packed_b));

                        // Unpack a: ptr_a = (packed_a >> 32) as i32, len_a = packed_a & 0xFFFFFFFF
                        ctx.emit(Instruction::LocalGet(packed_a));
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64ShrU);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64ExtendI32U);
                        ctx.emit(Instruction::LocalSet(ptr_a));

                        ctx.emit(Instruction::LocalGet(packed_a));
                        ctx.emit(Instruction::I64Const(0xFFFFFFFF));
                        ctx.emit(Instruction::I64And);
                        ctx.emit(Instruction::LocalSet(len_a));

                        // Unpack b: ptr_b, len_b
                        ctx.emit(Instruction::LocalGet(packed_b));
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64ShrU);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64ExtendI32U);
                        ctx.emit(Instruction::LocalSet(ptr_b));

                        ctx.emit(Instruction::LocalGet(packed_b));
                        ctx.emit(Instruction::I64Const(0xFFFFFFFF));
                        ctx.emit(Instruction::I64And);
                        ctx.emit(Instruction::LocalSet(len_b));

                        // Allocate new buffer: __alloc(len_a + len_b)
                        ctx.emit(Instruction::LocalGet(len_a));
                        ctx.emit(Instruction::LocalGet(len_b));
                        ctx.emit(Instruction::I64Add);
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "string_concat".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(new_ptr)); // new_ptr (i64, but value is i32-range)

                        // memory.copy(new_ptr, ptr_a, len_a)
                        ctx.emit(Instruction::LocalGet(new_ptr));
                        ctx.emit(Instruction::I32WrapI64); // dest
                        ctx.emit(Instruction::LocalGet(ptr_a));
                        ctx.emit(Instruction::I32WrapI64); // src
                        ctx.emit(Instruction::LocalGet(len_a));
                        ctx.emit(Instruction::I32WrapI64); // size
                        ctx.emit(Instruction::MemoryCopy {
                            src_mem: 0,
                            dst_mem: 0,
                        });

                        // memory.copy(new_ptr + len_a, ptr_b, len_b)
                        ctx.emit(Instruction::LocalGet(new_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(len_a));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add); // dest = new_ptr + len_a
                        ctx.emit(Instruction::LocalGet(ptr_b));
                        ctx.emit(Instruction::I32WrapI64); // src
                        ctx.emit(Instruction::LocalGet(len_b));
                        ctx.emit(Instruction::I32WrapI64); // size
                        ctx.emit(Instruction::MemoryCopy {
                            src_mem: 0,
                            dst_mem: 0,
                        });

                        // Pack result: (new_ptr << 32) | (len_a + len_b)
                        ctx.emit(Instruction::LocalGet(new_ptr));
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64Shl);
                        ctx.emit(Instruction::LocalGet(len_a));
                        ctx.emit(Instruction::LocalGet(len_b));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I64Or);
                    }

                    // string_eq(a, b) → 1 if equal, 0 if not
                    "string_eq" | "intrinsic_string_eq" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "string_eq requires 2 arguments".to_string(),
                                context: "string_eq".to_string(),
                            });
                        }
                        let packed_a = ctx.scope.get_or_alloc("__streq_a");
                        let packed_b = ctx.scope.get_or_alloc("__streq_b");
                        let ptr_a = ctx.scope.get_or_alloc("__streq_ptr_a");
                        let len_a = ctx.scope.get_or_alloc("__streq_len_a");
                        let ptr_b = ctx.scope.get_or_alloc("__streq_ptr_b");
                        let len_b = ctx.scope.get_or_alloc("__streq_len_b");
                        let idx = ctx.scope.get_or_alloc("__streq_idx");
                        let result = ctx.scope.get_or_alloc("__streq_result");

                        // Evaluate args
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(packed_a));
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::LocalSet(packed_b));

                        // Unpack a
                        ctx.emit(Instruction::LocalGet(packed_a));
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64ShrU);
                        ctx.emit(Instruction::LocalSet(ptr_a));

                        ctx.emit(Instruction::LocalGet(packed_a));
                        ctx.emit(Instruction::I64Const(0xFFFFFFFF));
                        ctx.emit(Instruction::I64And);
                        ctx.emit(Instruction::LocalSet(len_a));

                        // Unpack b
                        ctx.emit(Instruction::LocalGet(packed_b));
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64ShrU);
                        ctx.emit(Instruction::LocalSet(ptr_b));

                        ctx.emit(Instruction::LocalGet(packed_b));
                        ctx.emit(Instruction::I64Const(0xFFFFFFFF));
                        ctx.emit(Instruction::I64And);
                        ctx.emit(Instruction::LocalSet(len_b));

                        // Default result = 1 (assume equal)
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::LocalSet(result));

                        // If len_a != len_b → result = 0, skip loop
                        ctx.emit(Instruction::LocalGet(len_a));
                        ctx.emit(Instruction::LocalGet(len_b));
                        ctx.emit(Instruction::I64Ne);
                        ctx.emit(Instruction::If(BlockType::Empty));
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::LocalSet(result));
                        ctx.emit(Instruction::Else);

                        // Byte-by-byte comparison loop
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::LocalSet(idx));

                        ctx.emit(Instruction::Block(BlockType::Empty)); // outer block for break
                        ctx.emit(Instruction::Loop(BlockType::Empty));

                        // if idx >= len_a → break
                        ctx.emit(Instruction::LocalGet(idx));
                        ctx.emit(Instruction::LocalGet(len_a));
                        ctx.emit(Instruction::I64GeU);
                        ctx.emit(Instruction::BrIf(1)); // break out of block

                        // Compare bytes: mem[ptr_a + idx] vs mem[ptr_b + idx]
                        ctx.emit(Instruction::LocalGet(ptr_a));
                        ctx.emit(Instruction::LocalGet(idx));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Load8U(wasm_encoder::MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        }));

                        ctx.emit(Instruction::LocalGet(ptr_b));
                        ctx.emit(Instruction::LocalGet(idx));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Load8U(wasm_encoder::MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        }));

                        ctx.emit(Instruction::I32Ne);
                        ctx.emit(Instruction::If(BlockType::Empty));
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::LocalSet(result));
                        ctx.emit(Instruction::Br(2)); // break out of outer block
                        ctx.emit(Instruction::End); // end if

                        // idx += 1
                        ctx.emit(Instruction::LocalGet(idx));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::LocalSet(idx));

                        ctx.emit(Instruction::Br(0)); // continue loop
                        ctx.emit(Instruction::End); // end loop
                        ctx.emit(Instruction::End); // end block

                        ctx.emit(Instruction::End); // end else (len check)

                        // Push result
                        ctx.emit(Instruction::LocalGet(result));
                    }

                    // string_slice(s, start, end) → zero-copy substring
                    "string_slice" | "intrinsic_string_slice" => {
                        if args.len() != 3 {
                            return Err(WasmCompileError {
                                message: "string_slice requires 3 arguments (str, start, end)"
                                    .to_string(),
                                context: "string_slice".to_string(),
                            });
                        }
                        let packed = ctx.scope.get_or_alloc("__str_slice_packed");
                        let start = ctx.scope.get_or_alloc("__str_slice_start");
                        let end = ctx.scope.get_or_alloc("__str_slice_end");

                        // Evaluate args
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(packed));
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::LocalSet(start));
                        Self::compile_expr(ctx, &args[2], func_map)?;
                        ctx.emit(Instruction::LocalSet(end));

                        // Extract original ptr: (packed >> 32)
                        ctx.emit(Instruction::LocalGet(packed));
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64ShrU);
                        // new_ptr = original_ptr + start
                        ctx.emit(Instruction::LocalGet(start));
                        ctx.emit(Instruction::I64Add);
                        // shift left 32 to pack as high bits
                        ctx.emit(Instruction::I64Const(32));
                        ctx.emit(Instruction::I64Shl);
                        // new_len = end - start
                        ctx.emit(Instruction::LocalGet(end));
                        ctx.emit(Instruction::LocalGet(start));
                        ctx.emit(Instruction::I64Sub);
                        // Pack: (new_ptr << 32) | new_len
                        ctx.emit(Instruction::I64Or);
                    }

                    // =========================================================
                    // Tier 1: List & Struct intrinsics (linear memory ops)
                    // =========================================================

                    // len(list_or_struct) → read length/field_count header at ptr[0]
                    "len" | "intrinsic_len" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "len requires 1 argument".to_string(),
                                context: "intrinsic_len".to_string(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64); // ptr
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }

                    // list.get(list, index) → load list[index]
                    // ptr + 8 + 8*index
                    "intrinsic_list_get" | "sys.list.get" | "list.get" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "list.get requires 2 arguments (list, index)".to_string(),
                                context: "intrinsic_list_get".to_string(),
                            });
                        }
                        // Compile list ptr
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        let list_ptr = ctx.scope.get_or_alloc("__intrinsic_list_ptr");
                        ctx.emit(Instruction::LocalSet(list_ptr));

                        // Compute address: ptr + 8 + 8*index
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        // Calculate offset: 8 + 8 * index
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add); // base + offset

                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }

                    // list.set(list, index, value) → store value at list[index]
                    "intrinsic_list_set" | "sys.list.set" | "list.set" => {
                        if args.len() != 3 {
                            return Err(WasmCompileError {
                                message: "list.set requires 3 arguments (list, index, value)"
                                    .to_string(),
                                context: "intrinsic_list_set".to_string(),
                            });
                        }
                        let list_ptr = ctx.scope.get_or_alloc("__intrinsic_list_ptr");
                        let set_val = ctx.scope.get_or_alloc("__intrinsic_set_val");

                        // Compile list ptr
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(list_ptr));

                        // Compile value (3rd arg) first, stash it
                        Self::compile_expr(ctx, &args[2], func_map)?;
                        ctx.emit(Instruction::LocalSet(set_val));

                        // Compute store address: ptr + 8 + 8*index
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add); // addr

                        // Store value
                        ctx.emit(Instruction::LocalGet(set_val));
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Return the list ptr (for chaining)
                        ctx.emit(Instruction::LocalGet(list_ptr));
                    }

                    // list.append(list, value) → allocate new list with len+1,
                    // copy old data, store new value at end, return new ptr
                    "intrinsic_list_append" | "sys.list.append" | "list.append" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "list.append requires 2 arguments (list, value)"
                                    .to_string(),
                                context: "intrinsic_list_append".to_string(),
                            });
                        }
                        let old_ptr = ctx.scope.get_or_alloc("__append_old_ptr");
                        let old_len = ctx.scope.get_or_alloc("__append_old_len");
                        let new_ptr = ctx.scope.get_or_alloc("__append_new_ptr");
                        let append_val = ctx.scope.get_or_alloc("__append_val");
                        let copy_i = ctx.scope.get_or_alloc("__append_i");

                        // Compile old list ptr
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(old_ptr));

                        // Compile append value
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::LocalSet(append_val));

                        // Read old length
                        ctx.emit(Instruction::LocalGet(old_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        ctx.emit(Instruction::LocalSet(old_len));

                        // Allocate new list: 8 + 8*(old_len + 1)
                        ctx.emit(Instruction::LocalGet(old_len));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add); // total size
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "list.append".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(new_ptr));

                        // Store new length = old_len + 1
                        ctx.emit(Instruction::LocalGet(new_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(old_len));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Copy old elements: for i in 0..old_len
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::LocalSet(copy_i));

                        // block { loop {
                        ctx.emit(Instruction::Block(BlockType::Empty));
                        ctx.emit(Instruction::Loop(BlockType::Empty));

                        // if i >= old_len, break
                        ctx.emit(Instruction::LocalGet(copy_i));
                        ctx.emit(Instruction::LocalGet(old_len));
                        ctx.emit(Instruction::I64GeS);
                        ctx.emit(Instruction::BrIf(1));

                        // new_ptr[8 + 8*i] = old_ptr[8 + 8*i]
                        // dst address
                        ctx.emit(Instruction::LocalGet(new_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(copy_i));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add); // dst addr

                        // src value: old_ptr[8 + 8*i]
                        ctx.emit(Instruction::LocalGet(old_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(copy_i));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add); // src addr
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // store
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // i += 1
                        ctx.emit(Instruction::LocalGet(copy_i));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::LocalSet(copy_i));

                        ctx.emit(Instruction::Br(0)); // continue loop
                        ctx.emit(Instruction::End); // end loop
                        ctx.emit(Instruction::End); // end block

                        // Store appended value at new_ptr[8 + 8*old_len]
                        ctx.emit(Instruction::LocalGet(new_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(old_len));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalGet(append_val));
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Return new list ptr
                        ctx.emit(Instruction::LocalGet(new_ptr));
                    }

                    // list.pop(list[, index]) → decrement length, return removed element
                    // Without index: removes last element
                    "intrinsic_list_pop" | "sys.list.pop" | "list.pop" => {
                        if args.is_empty() {
                            return Err(WasmCompileError {
                                message: "list.pop requires at least 1 argument (list)".to_string(),
                                context: "intrinsic_list_pop".to_string(),
                            });
                        }
                        let list_ptr = ctx.scope.get_or_alloc("__pop_list_ptr");
                        let old_len = ctx.scope.get_or_alloc("__pop_old_len");

                        // Compile list ptr
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(list_ptr));

                        // Read old length
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        ctx.emit(Instruction::LocalSet(old_len));

                        // Read last element: ptr[8 + 8*(len-1)]
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(old_len));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Sub);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Decrement length in-place
                        let popped = ctx.scope.get_or_alloc("__pop_result");
                        ctx.emit(Instruction::LocalSet(popped));

                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(old_len));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Sub);
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Return popped value
                        ctx.emit(Instruction::LocalGet(popped));
                    }

                    // list.delete(list, index) → shift elements left, decrement length
                    "intrinsic_list_delete" | "sys.list.delete" | "list.delete" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "list.delete requires 2 arguments (list, index)"
                                    .to_string(),
                                context: "intrinsic_list_delete".to_string(),
                            });
                        }
                        let list_ptr = ctx.scope.get_or_alloc("__del_list_ptr");
                        let del_idx = ctx.scope.get_or_alloc("__del_idx");
                        let del_len = ctx.scope.get_or_alloc("__del_len");
                        let del_i = ctx.scope.get_or_alloc("__del_i");

                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(list_ptr));
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::LocalSet(del_idx));

                        // Read length
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        ctx.emit(Instruction::LocalSet(del_len));

                        // Shift elements left: for i = index..len-1: list[i] = list[i+1]
                        ctx.emit(Instruction::LocalGet(del_idx));
                        ctx.emit(Instruction::LocalSet(del_i));

                        ctx.emit(Instruction::Block(BlockType::Empty));
                        ctx.emit(Instruction::Loop(BlockType::Empty));

                        // if i >= len - 1, break
                        ctx.emit(Instruction::LocalGet(del_i));
                        ctx.emit(Instruction::LocalGet(del_len));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Sub);
                        ctx.emit(Instruction::I64GeS);
                        ctx.emit(Instruction::BrIf(1));

                        // list[i] = list[i+1]
                        // dst: ptr + 8 + 8*i
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(del_i));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);

                        // src: ptr + 8 + 8*(i+1)
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(del_i));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // i += 1
                        ctx.emit(Instruction::LocalGet(del_i));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::LocalSet(del_i));

                        ctx.emit(Instruction::Br(0));
                        ctx.emit(Instruction::End); // loop
                        ctx.emit(Instruction::End); // block

                        // Decrement length
                        ctx.emit(Instruction::LocalGet(list_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(del_len));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::I64Sub);
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Return list ptr
                        ctx.emit(Instruction::LocalGet(list_ptr));
                    }

                    // struct.get(struct, field_index) → load struct[field_index]
                    "intrinsic_struct_get" | "sys.struct.get" | "struct.get" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "struct.get requires 2 arguments (struct, field_index)"
                                    .to_string(),
                                context: "intrinsic_struct_get".to_string(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        let s_ptr = ctx.scope.get_or_alloc("__sget_ptr");
                        ctx.emit(Instruction::LocalSet(s_ptr));

                        ctx.emit(Instruction::LocalGet(s_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);

                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }

                    // struct.set(struct, field_index, value) → store at struct[field_index]
                    "intrinsic_struct_set" | "sys.struct.set" | "struct.set" => {
                        if args.len() != 3 {
                            return Err(WasmCompileError {
                                message:
                                    "struct.set requires 3 arguments (struct, field_index, value)"
                                        .to_string(),
                                context: "intrinsic_struct_set".to_string(),
                            });
                        }
                        let s_ptr = ctx.scope.get_or_alloc("__sset_ptr");
                        let s_val = ctx.scope.get_or_alloc("__sset_val");

                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(s_ptr));
                        Self::compile_expr(ctx, &args[2], func_map)?;
                        ctx.emit(Instruction::LocalSet(s_val));

                        // Compute address: ptr + 8 + 8*field_index
                        ctx.emit(Instruction::LocalGet(s_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Add);
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Add);

                        ctx.emit(Instruction::LocalGet(s_val));
                        ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Return struct ptr
                        ctx.emit(Instruction::LocalGet(s_ptr));
                    }

                    // struct.has(struct, field_index) → 1 if field_index < field_count, else 0
                    "intrinsic_struct_has" | "sys.struct.has" | "struct.has" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "struct.has requires 2 arguments (struct, field_index)"
                                    .to_string(),
                                context: "intrinsic_struct_has".to_string(),
                            });
                        }
                        // Read field_count from header
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));

                        // Compare: field_index < field_count → 1, else 0
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I64GtS); // field_count > field_index
                        ctx.emit(Instruction::I64ExtendI32U); // bool → i64
                    }

                    // =========================================================
                    // Tier 2: WASI-backed intrinsics
                    // =========================================================

                    // sys.time.now() → nanoseconds since epoch as i64
                    // Uses: clock_time_get(clock_id=0 (realtime), precision=1, timestamp_ptr)
                    "intrinsic_time_now" | "time.now" | "sys.time.now" => {
                        let ts_ptr = ctx.scope.get_or_alloc("__time_ts_ptr");

                        // Allocate 8 bytes for the timestamp result
                        ctx.emit(Instruction::I64Const(8));
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "sys.time.now".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(ts_ptr));

                        // Call clock_time_get(0, 1, ts_ptr)
                        ctx.emit(Instruction::I32Const(0)); // clock_id = REALTIME
                        ctx.emit(Instruction::I64Const(1)); // precision = 1ns
                        ctx.emit(Instruction::LocalGet(ts_ptr));
                        ctx.emit(Instruction::I32WrapI64); // ptr as i32
                        ctx.emit(Instruction::Call(WASI_CLOCK_TIME_GET_FUNC_IDX));
                        ctx.emit(Instruction::Drop); // drop errno

                        // Load the 8-byte timestamp
                        ctx.emit(Instruction::LocalGet(ts_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }

                    // sys.io.read_line() → reads from stdin (fd=0) into buffer, returns i64 bytes read
                    // Uses: fd_read(fd=0, iovs_ptr, iovs_len=1, nread_ptr) -> errno
                    "intrinsic_io_read_line" | "sys.io.read_line" => {
                        let buf_ptr = ctx.scope.get_or_alloc("__read_buf_ptr");
                        let iov_ptr = ctx.scope.get_or_alloc("__read_iov_ptr");
                        let nread_ptr = ctx.scope.get_or_alloc("__read_nread_ptr");

                        // Allocate 1024 bytes for read buffer
                        ctx.emit(Instruction::I64Const(1024));
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "sys.io.read_line".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(buf_ptr));

                        // Allocate 8 bytes for iovec: [buf_ptr:i32, buf_len:i32]
                        ctx.emit(Instruction::I64Const(8));
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "sys.io.read_line iov".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(iov_ptr));

                        // Allocate 4 bytes for nread
                        ctx.emit(Instruction::I64Const(4));
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "sys.io.read_line nread".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(nread_ptr));

                        // Fill iovec: iov[0] = buf_ptr, iov[4] = 1024
                        ctx.emit(Instruction::LocalGet(iov_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(buf_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                        ctx.emit(Instruction::LocalGet(iov_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(1024));
                        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 0,
                        }));

                        // Call fd_read(0, iov_ptr, 1, nread_ptr)
                        ctx.emit(Instruction::I32Const(0)); // fd = stdin
                        ctx.emit(Instruction::LocalGet(iov_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(1)); // iovs_len
                        ctx.emit(Instruction::LocalGet(nread_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::Call(WASI_FD_READ_FUNC_IDX));
                        ctx.emit(Instruction::Drop); // drop errno

                        // Return nread as i64
                        ctx.emit(Instruction::LocalGet(nread_ptr));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Load(wasm_encoder::MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                        ctx.emit(Instruction::I64ExtendI32U);
                    }

                    // sys.crypto.random_bytes(n) → fills buffer with n random bytes, returns ptr as i64
                    // Uses: random_get(buf_ptr, buf_len) -> errno
                    "intrinsic_crypto_random_bytes" | "sys.crypto.random_bytes" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "sys.crypto.random_bytes requires 1 argument (byte_count)"
                                    .to_string(),
                                context: "sys.crypto.random_bytes".to_string(),
                            });
                        }
                        let rand_len = ctx.scope.get_or_alloc("__rand_len");
                        let rand_buf = ctx.scope.get_or_alloc("__rand_buf");

                        // Compile byte count
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::LocalSet(rand_len));

                        // Allocate buffer
                        ctx.emit(Instruction::LocalGet(rand_len));
                        if let Some(&alloc_idx) = func_map.get("__alloc") {
                            ctx.emit(Instruction::Call(alloc_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: "__alloc not found".to_string(),
                                context: "sys.crypto.random_bytes".to_string(),
                            });
                        }
                        ctx.emit(Instruction::LocalSet(rand_buf));

                        // Call random_get(buf_ptr, buf_len)
                        ctx.emit(Instruction::LocalGet(rand_buf));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(rand_len));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::Call(WASI_RANDOM_GET_FUNC_IDX));
                        ctx.emit(Instruction::Drop); // drop errno

                        // Return buffer ptr as i64
                        ctx.emit(Instruction::LocalGet(rand_buf));
                    }

                    // sys.exit(code?) → proc_exit, never returns
                    "sys.exit" | "exit" | "intrinsic_exit" => {
                        if args.is_empty() {
                            ctx.emit(Instruction::I32Const(0)); // default exit code
                        } else {
                            Self::compile_expr(ctx, &args[0], func_map)?;
                            ctx.emit(Instruction::I32WrapI64);
                        }
                        ctx.emit(Instruction::Call(WASI_PROC_EXIT_FUNC_IDX));
                        // proc_exit never returns, but we need a value on stack for WASM validation
                        ctx.emit(Instruction::Unreachable);
                    }

                    // fd_close(fd) → close file descriptor
                    "fd_close" | "sys.io.close" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "fd_close requires 1 argument (fd)".to_string(),
                                context: "fd_close".to_string(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::Call(WASI_FD_CLOSE_FUNC_IDX));
                        ctx.emit(Instruction::I64ExtendI32U); // errno as i64
                    }

                    // =============================================================
                    // Tier 3: Ark Host Import Intrinsics
                    // =============================================================

                    // --- Math unary: f64 reinterpret via i64 ---
                    // math.sin(x), math.cos(x), math.tan(x)
                    // math.asin(x), math.acos(x), math.atan(x), math.sqrt(x)
                    "intrinsic_math_sin" | "math.sin" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.sin needs 1 arg".into(),
                                context: "math.sin".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_SIN_FUNC_IDX));
                    }
                    "intrinsic_math_cos" | "math.cos" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.cos needs 1 arg".into(),
                                context: "math.cos".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_COS_FUNC_IDX));
                    }
                    "intrinsic_math_tan" | "math.tan" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.tan needs 1 arg".into(),
                                context: "math.tan".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_TAN_FUNC_IDX));
                    }
                    "intrinsic_math_asin" | "math.asin" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.asin needs 1 arg".into(),
                                context: "math.asin".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_ASIN_FUNC_IDX));
                    }
                    "intrinsic_math_acos" | "math.acos" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.acos needs 1 arg".into(),
                                context: "math.acos".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_ACOS_FUNC_IDX));
                    }
                    "intrinsic_math_atan" | "math.atan" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.atan needs 1 arg".into(),
                                context: "math.atan".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_ATAN_FUNC_IDX));
                    }
                    "intrinsic_math_sqrt" | "math.sqrt" => {
                        if args.len() != 1 {
                            return Err(WasmCompileError {
                                message: "math.sqrt needs 1 arg".into(),
                                context: "math.sqrt".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_SQRT_FUNC_IDX));
                    }

                    // --- Math binary ---
                    "intrinsic_math_atan2" | "math.atan2" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "math.atan2 needs 2 args".into(),
                                context: "math.atan2".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?; // y
                        Self::compile_expr(ctx, &args[1], func_map)?; // x
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_ATAN2_FUNC_IDX));
                    }
                    "intrinsic_math_pow" | "math.pow" => {
                        if args.len() != 2 {
                            return Err(WasmCompileError {
                                message: "math.pow needs 2 args".into(),
                                context: "math.pow".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?; // base
                        Self::compile_expr(ctx, &args[1], func_map)?; // exp
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_POW_FUNC_IDX));
                    }

                    // --- Math ternary ---
                    "intrinsic_pow_mod" | "math.pow_mod" | "sys.math.pow_mod" => {
                        if args.len() != 3 {
                            return Err(WasmCompileError {
                                message: "pow_mod needs 3 args".into(),
                                context: "pow_mod".into(),
                            });
                        }
                        Self::compile_expr(ctx, &args[0], func_map)?; // base
                        Self::compile_expr(ctx, &args[1], func_map)?; // exp
                        Self::compile_expr(ctx, &args[2], func_map)?; // modulus
                        ctx.emit(Instruction::Call(ARK_HOST_MATH_POW_MOD_FUNC_IDX));
                    }

                    // --- Crypto: SHA-512 ---
                    // sys.crypto.sha512(data_ptr) => allocates 64-byte output, returns ptr as i64
                    "intrinsic_crypto_sha512" | "sys.crypto.sha512" => {
                        // Arg is the data pointer (packed: upper 32 = ptr, lower 32 = len)
                        // For simplicity: arg[0] = ptr (i64), arg[1] = len (i64)
                        // Host call: crypto_sha512(data_ptr:i32, data_len:i32, out_ptr:i32) -> i32
                        if args.len() < 2 {
                            return Err(WasmCompileError {
                                message: "sha512 needs 2 args (ptr, len)".to_string(),
                                context: "sha512".to_string(),
                            });
                        }
                        // Allocate 64 bytes for output
                        ctx.emit(Instruction::I64Const(64));
                        let alloc_idx = *func_map.get("__alloc").unwrap();
                        ctx.emit(Instruction::Call(alloc_idx));
                        // out_ptr is on stack as i64; save to local
                        let out_local = ctx.scope.get_or_alloc("__host_out_sha512");
                        ctx.emit(Instruction::LocalSet(out_local));

                        // Push args: data_ptr (i32), data_len (i32), out_ptr (i32)
                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64); // data_ptr
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I32WrapI64); // data_len
                        ctx.emit(Instruction::LocalGet(out_local));
                        ctx.emit(Instruction::I32WrapI64); // out_ptr

                        ctx.emit(Instruction::Call(ARK_HOST_CRYPTO_SHA512_FUNC_IDX));
                        ctx.emit(Instruction::Drop); // drop errno
                        ctx.emit(Instruction::LocalGet(out_local)); // return out_ptr as i64
                    }

                    // --- JSON ---
                    "sys.json.parse" | "intrinsic_json_parse" => {
                        if args.len() < 2 {
                            return Err(WasmCompileError {
                                message: "json.parse needs 2 args (str_ptr, str_len)".to_string(),
                                context: "json.parse".to_string(),
                            });
                        }
                        // Allocate output buffer (4KB)
                        ctx.emit(Instruction::I64Const(4096));
                        let alloc_idx = *func_map.get("__alloc").unwrap();
                        ctx.emit(Instruction::Call(alloc_idx));
                        let out_local = ctx.scope.get_or_alloc("__host_out_sha512");
                        ctx.emit(Instruction::LocalSet(out_local));

                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64);
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(out_local));
                        ctx.emit(Instruction::I32WrapI64);

                        ctx.emit(Instruction::Call(ARK_HOST_JSON_PARSE_FUNC_IDX));
                        ctx.emit(Instruction::I64ExtendI32U); // bytes written as i64
                    }

                    "sys.json.stringify" | "intrinsic_json_stringify" => {
                        if args.len() < 2 {
                            return Err(WasmCompileError {
                                message: "json.stringify needs 2 args (val_ptr, val_len)"
                                    .to_string(),
                                context: "json.stringify".to_string(),
                            });
                        }
                        ctx.emit(Instruction::I64Const(4096));
                        let alloc_idx = *func_map.get("__alloc").unwrap();
                        ctx.emit(Instruction::Call(alloc_idx));
                        let out_local = ctx.scope.get_or_alloc("__host_out_sha512");
                        ctx.emit(Instruction::LocalSet(out_local));

                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64);
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(out_local));
                        ctx.emit(Instruction::I32WrapI64);

                        ctx.emit(Instruction::Call(ARK_HOST_JSON_STRINGIFY_FUNC_IDX));
                        ctx.emit(Instruction::I64ExtendI32U);
                    }

                    // --- AI ---
                    "intrinsic_ask_ai" | "sys.ai.ask" | "ai.ask" => {
                        // ask_ai(prompt_ptr, prompt_len, out_ptr, out_cap) -> bytes_written
                        if args.len() < 2 {
                            return Err(WasmCompileError {
                                message: "ask_ai needs 2 args (prompt_ptr, prompt_len)".to_string(),
                                context: "ask_ai".to_string(),
                            });
                        }
                        // Allocate 4KB output buffer
                        ctx.emit(Instruction::I64Const(4096));
                        let alloc_idx = *func_map.get("__alloc").unwrap();
                        ctx.emit(Instruction::Call(alloc_idx));
                        let out_local = ctx.scope.get_or_alloc("__host_out_sha512");
                        ctx.emit(Instruction::LocalSet(out_local));

                        Self::compile_expr(ctx, &args[0], func_map)?;
                        ctx.emit(Instruction::I32WrapI64); // prompt_ptr
                        Self::compile_expr(ctx, &args[1], func_map)?;
                        ctx.emit(Instruction::I32WrapI64); // prompt_len
                        ctx.emit(Instruction::LocalGet(out_local));
                        ctx.emit(Instruction::I32WrapI64); // out_ptr
                        ctx.emit(Instruction::I32Const(4096)); // out_cap

                        ctx.emit(Instruction::Call(ARK_HOST_ASK_AI_FUNC_IDX));
                        ctx.emit(Instruction::I64ExtendI32U); // bytes written as i64
                    }

                    _ => {
                        // Compile arguments
                        for arg in args {
                            Self::compile_expr(ctx, arg, func_map)?;
                        }
                        // Look up function index
                        if let Some(&func_idx) = func_map.get(function_hash) {
                            ctx.emit(Instruction::Call(func_idx));
                        } else {
                            return Err(WasmCompileError {
                                message: format!("Unknown function: {}", function_hash),
                                context: "compile_expr::Call".to_string(),
                            });
                        }
                    }
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // List literal → allocate in linear memory via __alloc
            // Layout: [length: i64 (8B)] [elem_0: i64 (8B)] [elem_1: i64 (8B)] ...
            // Returns: ptr as i64
            // -----------------------------------------------------------------
            Expression::List(items) => {
                let list_len = items.len();
                let alloc_size = 8 + 8 * list_len; // header + elements

                // Call __alloc(size) — __alloc is in func_index_map
                ctx.emit(Instruction::I64Const(alloc_size as i64));
                if let Some(&alloc_idx) = func_map.get("__alloc") {
                    ctx.emit(Instruction::Call(alloc_idx));
                } else {
                    return Err(WasmCompileError {
                        message: "__alloc not found — bump allocator not registered".to_string(),
                        context: "Expression::List".to_string(),
                    });
                }

                // Store returned ptr in a local
                let list_ptr = ctx.scope.get_or_alloc("__list_ptr");
                ctx.emit(Instruction::LocalSet(list_ptr));

                // Store length at ptr[0] (i64)
                ctx.emit(Instruction::LocalGet(list_ptr));
                ctx.emit(Instruction::I32WrapI64); // memory address must be i32
                ctx.emit(Instruction::I64Const(list_len as i64));
                ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                    offset: 0,
                    align: 3, // 8-byte aligned
                    memory_index: 0,
                }));

                // Store each element at ptr + 8 + 8*i
                for (i, item) in items.iter().enumerate() {
                    ctx.emit(Instruction::LocalGet(list_ptr));
                    ctx.emit(Instruction::I32WrapI64);
                    Self::compile_expr(ctx, item, func_map)?;
                    ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                        offset: (8 + 8 * i) as u64,
                        align: 3,
                        memory_index: 0,
                    }));
                }

                // Push ptr as the list value
                ctx.emit(Instruction::LocalGet(list_ptr));
                Ok(())
            }

            // -----------------------------------------------------------------
            // Struct init → allocate in linear memory via __alloc
            // Layout: [field_count: i64 (8B)] [field_0: i64] [field_1: i64] ...
            // Returns: ptr as i64
            // -----------------------------------------------------------------
            Expression::StructInit { fields } => {
                let field_count = fields.len();
                let alloc_size = 8 + 8 * field_count;

                ctx.emit(Instruction::I64Const(alloc_size as i64));
                if let Some(&alloc_idx) = func_map.get("__alloc") {
                    ctx.emit(Instruction::Call(alloc_idx));
                } else {
                    return Err(WasmCompileError {
                        message: "__alloc not found".to_string(),
                        context: "Expression::StructInit".to_string(),
                    });
                }

                let struct_ptr = ctx.scope.get_or_alloc("__struct_ptr");
                ctx.emit(Instruction::LocalSet(struct_ptr));

                // Store field count at ptr[0]
                ctx.emit(Instruction::LocalGet(struct_ptr));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I64Const(field_count as i64));
                ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));

                // Store each field value at ptr + 8 + 8*i
                for (i, (_name, value)) in fields.iter().enumerate() {
                    ctx.emit(Instruction::LocalGet(struct_ptr));
                    ctx.emit(Instruction::I32WrapI64);
                    Self::compile_expr(ctx, value, func_map)?;
                    ctx.emit(Instruction::I64Store(wasm_encoder::MemArg {
                        offset: (8 + 8 * i) as u64,
                        align: 3,
                        memory_index: 0,
                    }));
                }

                ctx.emit(Instruction::LocalGet(struct_ptr));
                Ok(())
            }

            // -----------------------------------------------------------------
            // Field access → load from linear memory
            // Struct fields are stored sequentially starting at ptr+8.
            // We use compile-time field name → index mapping.
            // For now, field index is determined by convention:
            //   - We check the obj expression to determine struct shape
            //   - Fall back to returning 0 for unknown fields
            // -----------------------------------------------------------------
            Expression::GetField { obj, field } => {
                // Compile the object (should return a ptr as i64)
                Self::compile_expr(ctx, obj, func_map)?;

                let obj_ptr = ctx.scope.get_or_alloc("__getfield_ptr");
                ctx.emit(Instruction::LocalSet(obj_ptr));

                // Special case: ".length" on lists → read header at ptr[0]
                if field == "length" || field == "len" {
                    ctx.emit(Instruction::LocalGet(obj_ptr));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                } else {
                    // For struct fields, use field name hash to determine index.
                    // This is a simple sequential scheme: field at index i is at
                    // ptr + 8 + 8*i. Without type info, we use a name-based
                    // lookup approach where the first field is index 0.
                    // For now, emit a load from field index 0 as default.
                    // Full field resolution requires passing struct type info.
                    ctx.emit(Instruction::LocalGet(obj_ptr));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load(wasm_encoder::MemArg {
                        offset: 8, // first field after header
                        align: 3,
                        memory_index: 0,
                    }));
                }
                Ok(())
            }

            // -----------------------------------------------------------------
            // Match expression → if/else chain
            // Compiles to: evaluate scrutinee, compare with each pattern,
            // first match executes its arm body.
            // -----------------------------------------------------------------
            Expression::Match { scrutinee, arms } => {
                // Compile scrutinee once, store in local
                Self::compile_expr(ctx, scrutinee, func_map)?;
                let match_val = ctx.scope.get_or_alloc("__match_val");
                ctx.emit(Instruction::LocalSet(match_val));

                // Generate nested if/else chain
                // For N arms, we generate N-1 if/else blocks, last arm is default
                if arms.is_empty() {
                    ctx.emit(Instruction::I64Const(0)); // Unit
                    return Ok(());
                }

                let last_idx = arms.len() - 1;
                for (i, (pattern, body)) in arms.iter().enumerate() {
                    let is_last = i == last_idx;

                    match pattern {
                        Pattern::Wildcard | Pattern::Variable(_) => {
                            // Wildcard/variable always matches — compile body
                            if let Pattern::Variable(name) = pattern {
                                let var_local = ctx.scope.get_or_alloc(name);
                                ctx.emit(Instruction::LocalGet(match_val));
                                ctx.emit(Instruction::LocalSet(var_local));
                            }
                            Self::compile_expr(ctx, body, func_map)?;
                            // Close any open if/else blocks from previous arms
                            for _ in 0..i {
                                ctx.emit(Instruction::End);
                            }
                            return Ok(());
                        }
                        Pattern::Literal(lit_str) => {
                            // Compare scrutinee to the literal value
                            ctx.emit(Instruction::LocalGet(match_val));
                            // Try to parse as integer
                            if let Ok(n) = lit_str.parse::<i64>() {
                                ctx.emit(Instruction::I64Const(n));
                            } else {
                                // String literal comparison — for now, use 0
                                ctx.emit(Instruction::I64Const(0));
                            }
                            ctx.emit(Instruction::I64Eq);

                            if is_last {
                                // Last arm: if matches, body; else unit
                                ctx.emit(Instruction::If(BlockType::Result(ValType::I64)));
                                Self::compile_expr(ctx, body, func_map)?;
                                ctx.emit(Instruction::Else);
                                ctx.emit(Instruction::I64Const(0)); // no match = Unit
                                ctx.emit(Instruction::End);
                            } else {
                                ctx.emit(Instruction::If(BlockType::Result(ValType::I64)));
                                Self::compile_expr(ctx, body, func_map)?;
                                ctx.emit(Instruction::Else);
                                // Next arm continues in the else branch
                            }
                        }
                        Pattern::EnumVariant { .. } => {
                            // Enum variant pattern: treat like wildcard for now
                            Self::compile_expr(ctx, body, func_map)?;
                            for _ in 0..i {
                                ctx.emit(Instruction::End);
                            }
                            return Ok(());
                        }
                    }
                }

                // Close remaining if/else blocks (for non-wildcard last arms)
                // Each Literal arm that's not last opens an if/else
                // The last Literal arm closes with End inside the loop
                // So we need (arms.len() - 1) End instructions
                for _ in 0..(arms.len().saturating_sub(1)) {
                    ctx.emit(Instruction::End);
                }

                Ok(())
            }

            // -----------------------------------------------------------------
            // Lambda expression → push table index as i64
            // Lambdas are registered during collect_lambdas and hoisted to
            // top-level synthetic functions (__lambda_N). The lambda's
            // table index is pushed onto the stack for call_indirect.
            // -----------------------------------------------------------------
            Expression::Lambda { params, body } => {
                // Look up all registered lambda names to find this one
                // Use a counter based on how many lambdas we've seen in
                // this function context so far
                let mut found_idx: Option<u32> = None;
                for idx in 0..100 {
                    let name = format!("__lambda_{}", idx);
                    if let Some(&func_idx) = func_map.get(&name) {
                        // Check if this is the right lambda by matching param count
                        // For now, just use the first unmatched lambda by index
                        found_idx = Some(func_idx);
                        break;
                    }
                }

                if let Some(func_idx) = found_idx {
                    // Push the function index as i64 — this can be used with
                    // call_indirect via the WASM Table
                    ctx.emit(Instruction::I64Const(func_idx as i64));
                } else {
                    // Lambda not pre-registered — emit inline block
                    let body_len = body.len();
                    if body_len == 0 {
                        ctx.emit(Instruction::I64Const(0));
                    } else {
                        for (i, stmt) in body.iter().enumerate() {
                            let is_last = i == body_len - 1;
                            Self::compile_stmt(ctx, stmt, is_last, func_map)?;
                        }
                    }
                }
                let _ = params; // params used during pre-registration
                Ok(())
            }
            Expression::EnumInit {
                enum_name: _,
                variant: _,
                args,
            } => {
                // WASM: enum values are represented as i64 tags
                // For now, push the number of fields as the tag
                if args.is_empty() {
                    ctx.emit(Instruction::I64Const(0));
                } else {
                    // Compile first field only (simplified WASM representation)
                    Self::compile_expr(ctx, &args[0], func_map)?;
                }
                Ok(())
            }
        }
    }

    // =========================================================================
    // Instruction Helpers
    // =========================================================================

    fn compile_binary_op(
        ctx: &mut FuncContext,
        args: &[Expression],
        op: Instruction<'static>,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        if args.len() != 2 {
            return Err(WasmCompileError {
                message: format!("Binary op requires 2 args, got {}", args.len()),
                context: "compile_binary_op".to_string(),
            });
        }
        Self::compile_expr(ctx, &args[0], func_map)?;
        Self::compile_expr(ctx, &args[1], func_map)?;
        ctx.emit(op);
        Ok(())
    }

    fn compile_compare_op(
        ctx: &mut FuncContext,
        args: &[Expression],
        op: Instruction<'static>,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        if args.len() != 2 {
            return Err(WasmCompileError {
                message: format!("Comparison requires 2 args, got {}", args.len()),
                context: "compile_compare_op".to_string(),
            });
        }
        Self::compile_expr(ctx, &args[0], func_map)?;
        Self::compile_expr(ctx, &args[1], func_map)?;
        ctx.emit(op);
        // Comparison returns i32 in WASM — extend to i64 for Ark's uniform type
        ctx.emit(Instruction::I64ExtendI32U);
        Ok(())
    }

    /// Compile `print(arg)` → type-dispatched output via WASI fd_write.
    ///
    /// Dispatches at compile time based on AST node type:
    /// - `Expression::Literal(s)` → string print (unpack ptr|len, fd_write)
    /// - Everything else → integer-to-ASCII conversion (itoa) + fd_write
    ///
    /// Memory layout (scratch region bytes 0–63):
    ///   0–23:  digit buffer (max 20 digits + sign + newline + padding)
    ///   24–24: newline byte for string print path
    ///   32–39: iovec { buf_ptr: i32, buf_len: i32 }
    ///   40–47: second iovec for newline { buf_ptr: i32, buf_len: i32 }
    ///   48–51: nwritten output
    fn compile_print(
        ctx: &mut FuncContext,
        args: &[Expression],
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        for arg in args {
            // Compile-time dispatch: string literals vs everything else
            match arg {
                Expression::Literal(_) => {
                    Self::compile_print_string(ctx, arg, func_map)?;
                }
                _ => {
                    Self::compile_print_integer(ctx, arg, func_map)?;
                }
            }
        }

        // Print returns Unit (0)
        ctx.emit(Instruction::I64Const(0));
        Ok(())
    }

    /// Compile string print: unpack packed ptr|len from i64, fd_write string + newline.
    ///
    /// String literals are packed as `(ptr << 32) | len` in a single i64.
    /// This function unpacks them, builds two iovecs (string + newline), and
    /// uses fd_write with iovs_len=2 to output them in a single syscall.
    fn compile_print_string(
        ctx: &mut FuncContext,
        arg: &Expression,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        // Compile the expression (pushes packed ptr|len as i64)
        Self::compile_expr(ctx, arg, func_map)?;

        let packed_local = ctx.scope.get_or_alloc("__print_val");
        ctx.emit(Instruction::LocalSet(packed_local));

        // Extract ptr = (packed >> 32) as i32
        // Extract len = (packed & 0xFFFFFFFF) as i32

        // Store newline byte at scratch offset 24
        ctx.emit(Instruction::I32Const(24)); // memory address
        ctx.emit(Instruction::I32Const(10)); // '\n'
        ctx.emit(Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));

        // Build iovec[0] at offset 32: { buf_ptr = ptr, buf_len = len }
        ctx.emit(Instruction::I32Const(32)); // iovec[0].buf_ptr address
        ctx.emit(Instruction::LocalGet(packed_local));
        ctx.emit(Instruction::I64Const(32));
        ctx.emit(Instruction::I64ShrU); // ptr = packed >> 32
        ctx.emit(Instruction::I32WrapI64); // as i32
        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        ctx.emit(Instruction::I32Const(36)); // iovec[0].buf_len address
        ctx.emit(Instruction::LocalGet(packed_local));
        ctx.emit(Instruction::I32WrapI64); // len = packed & 0xFFFFFFFF (low 32 bits)
        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Build iovec[1] at offset 40: { buf_ptr = 24, buf_len = 1 } (newline)
        ctx.emit(Instruction::I32Const(40)); // iovec[1].buf_ptr address
        ctx.emit(Instruction::I32Const(24)); // points to our '\n' byte
        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        ctx.emit(Instruction::I32Const(44)); // iovec[1].buf_len address
        ctx.emit(Instruction::I32Const(1)); // 1 byte (newline)
        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Call fd_write(fd=1, iovs=32, iovs_len=2, nwritten=48)
        ctx.emit(Instruction::I32Const(1)); // fd = stdout
        ctx.emit(Instruction::I32Const(32)); // iovs pointer (two iovecs)
        ctx.emit(Instruction::I32Const(2)); // iovs_len = 2 (string + newline)
        ctx.emit(Instruction::I32Const(48)); // nwritten pointer
        ctx.emit(Instruction::Call(0)); // fd_write is import index 0
        ctx.emit(Instruction::Drop); // drop fd_write return value

        Ok(())
    }

    /// Compile integer print: itoa conversion + fd_write.
    ///
    /// Converts an i64 value to its decimal ASCII string representation in
    /// linear memory, then writes it to stdout via fd_write. Handles negative
    /// numbers and appends a newline.
    fn compile_print_integer(
        ctx: &mut FuncContext,
        arg: &Expression,
        func_map: &HashMap<String, u32>,
    ) -> Result<(), WasmCompileError> {
        Self::compile_expr(ctx, arg, func_map)?;

        // We need several scratch locals:
        //   __print_val    : the i64 value to print
        //   __print_neg    : 1 if negative, 0 if positive (i64)
        //   __print_pos    : current write position in digit buffer (i64 used as i32)
        //   __print_digit  : temp for digit extraction
        let val_local = ctx.scope.get_or_alloc("__print_val");
        let neg_local = ctx.scope.get_or_alloc("__print_neg");
        let pos_local = ctx.scope.get_or_alloc("__print_pos");
        let digit_local = ctx.scope.get_or_alloc("__print_digit");

        // Store the expression result
        ctx.emit(Instruction::LocalSet(val_local));

        // Initialize: not negative
        ctx.emit(Instruction::I64Const(0));
        ctx.emit(Instruction::LocalSet(neg_local));

        // Start writing from end of buffer (position 20)
        ctx.emit(Instruction::I64Const(20));
        ctx.emit(Instruction::LocalSet(pos_local));

        // Handle sign: if val < 0, negate it and set neg flag
        // if (val < 0) { neg = 1; val = -val; }
        ctx.emit(Instruction::LocalGet(val_local));
        ctx.emit(Instruction::I64Const(0));
        ctx.emit(Instruction::I64LtS);
        ctx.emit(Instruction::If(BlockType::Empty));
        {
            ctx.emit(Instruction::I64Const(1));
            ctx.emit(Instruction::LocalSet(neg_local));
            // Negate: val = 0 - val
            ctx.emit(Instruction::I64Const(0));
            ctx.emit(Instruction::LocalGet(val_local));
            ctx.emit(Instruction::I64Sub);
            ctx.emit(Instruction::LocalSet(val_local));
        }
        ctx.emit(Instruction::End);

        // Handle special case: val == 0
        ctx.emit(Instruction::LocalGet(val_local));
        ctx.emit(Instruction::I64Const(0));
        ctx.emit(Instruction::I64Eq);
        ctx.emit(Instruction::If(BlockType::Empty));
        {
            // Write '0' at position 19 (consistent with non-zero branch)
            ctx.emit(Instruction::I32Const(19)); // memory address
            ctx.emit(Instruction::I32Const(48)); // ASCII '0'
            ctx.emit(Instruction::I32Store8(wasm_encoder::MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
            ctx.emit(Instruction::I64Const(19));
            ctx.emit(Instruction::LocalSet(pos_local));
        }
        ctx.emit(Instruction::Else);
        {
            // Loop: extract digits from val
            // while (val > 0) {
            //   pos--;
            //   mem[pos] = (val % 10) + 48;
            //   val /= 10;
            // }
            ctx.emit(Instruction::Block(BlockType::Empty)); // block for break
            ctx.emit(Instruction::Loop(BlockType::Empty)); // loop
            {
                // Check: if val == 0, break
                ctx.emit(Instruction::LocalGet(val_local));
                ctx.emit(Instruction::I64Const(0));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::BrIf(1)); // break out of block

                // pos--
                ctx.emit(Instruction::LocalGet(pos_local));
                ctx.emit(Instruction::I64Const(1));
                ctx.emit(Instruction::I64Sub);
                ctx.emit(Instruction::LocalSet(pos_local));

                // digit = val % 10
                ctx.emit(Instruction::LocalGet(val_local));
                ctx.emit(Instruction::I64Const(10));
                ctx.emit(Instruction::I64RemU);
                ctx.emit(Instruction::LocalSet(digit_local));

                // mem[pos] = digit + 48 (ASCII '0')
                ctx.emit(Instruction::LocalGet(pos_local));
                ctx.emit(Instruction::I32WrapI64); // pos as i32
                ctx.emit(Instruction::LocalGet(digit_local));
                ctx.emit(Instruction::I64Const(48));
                ctx.emit(Instruction::I64Add);
                ctx.emit(Instruction::I32WrapI64); // digit+48 as i32
                ctx.emit(Instruction::I32Store8(wasm_encoder::MemArg {
                    offset: 0,
                    align: 0,
                    memory_index: 0,
                }));

                // val /= 10
                ctx.emit(Instruction::LocalGet(val_local));
                ctx.emit(Instruction::I64Const(10));
                ctx.emit(Instruction::I64DivU);
                ctx.emit(Instruction::LocalSet(val_local));

                // Continue loop
                ctx.emit(Instruction::Br(0));
            }
            ctx.emit(Instruction::End); // end loop
            ctx.emit(Instruction::End); // end block
        }
        ctx.emit(Instruction::End); // end if/else

        // If negative, prepend '-'
        ctx.emit(Instruction::LocalGet(neg_local));
        ctx.emit(Instruction::I64Const(1));
        ctx.emit(Instruction::I64Eq);
        ctx.emit(Instruction::If(BlockType::Empty));
        {
            ctx.emit(Instruction::LocalGet(pos_local));
            ctx.emit(Instruction::I64Const(1));
            ctx.emit(Instruction::I64Sub);
            ctx.emit(Instruction::LocalSet(pos_local));
            // mem[pos] = '-' (ASCII 45)
            ctx.emit(Instruction::LocalGet(pos_local));
            ctx.emit(Instruction::I32WrapI64);
            ctx.emit(Instruction::I32Const(45)); // '-'
            ctx.emit(Instruction::I32Store8(wasm_encoder::MemArg {
                offset: 0,
                align: 0,
                memory_index: 0,
            }));
        }
        ctx.emit(Instruction::End);

        // Append newline at position 20 (right after digit area ending at 19)
        ctx.emit(Instruction::I32Const(20));
        ctx.emit(Instruction::I32Const(10)); // '\n'
        ctx.emit(Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));

        // Digits are at mem[pos..19], newline at mem[20]
        // Total output length = 21 - pos (includes newline)
        // Build iovec at offset 32: { buf_ptr: i32, buf_len: i32 }
        ctx.emit(Instruction::I32Const(32)); // iovec address
        ctx.emit(Instruction::LocalGet(pos_local));
        ctx.emit(Instruction::I32WrapI64); // buf_ptr = pos
        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // buf_len = 21 - pos
        ctx.emit(Instruction::I32Const(36)); // iovec + 4
        ctx.emit(Instruction::I32Const(21));
        ctx.emit(Instruction::LocalGet(pos_local));
        ctx.emit(Instruction::I32WrapI64);
        ctx.emit(Instruction::I32Sub); // 21 - pos
        ctx.emit(Instruction::I32Store(wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        }));

        // Call fd_write(fd=1, iovs=32, iovs_len=1, nwritten=48)
        ctx.emit(Instruction::I32Const(1)); // fd = stdout
        ctx.emit(Instruction::I32Const(32)); // iovs pointer
        ctx.emit(Instruction::I32Const(1)); // iovs count
        ctx.emit(Instruction::I32Const(48)); // nwritten pointer
        ctx.emit(Instruction::Call(0)); // fd_write is import index 0
        ctx.emit(Instruction::Drop); // drop fd_write return value

        Ok(())
    }

    // =========================================================================
    // Module Emission
    // =========================================================================

    fn emit_module(&self) -> Vec<u8> {
        let mut module = Module::new();

        // --- Type Section ---
        let mut types = TypeSection::new();
        for (params, results) in &self.types {
            types.ty().function(params.clone(), results.clone());
        }
        module.section(&types);

        // --- Import Section (WASI Preview1) ---
        let mut imports = ImportSection::new();
        // Index 0: fd_write
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            wasm_encoder::EntityType::Function(WASI_FD_WRITE_TYPE_IDX),
        );
        // Index 1: fd_read (same type as fd_write)
        imports.import(
            "wasi_snapshot_preview1",
            "fd_read",
            wasm_encoder::EntityType::Function(WASI_FD_WRITE_TYPE_IDX),
        );
        // Index 2: clock_time_get
        imports.import(
            "wasi_snapshot_preview1",
            "clock_time_get",
            wasm_encoder::EntityType::Function(WASI_CLOCK_TIME_GET_TYPE_IDX),
        );
        // Index 3: random_get
        imports.import(
            "wasi_snapshot_preview1",
            "random_get",
            wasm_encoder::EntityType::Function(WASI_RANDOM_GET_TYPE_IDX),
        );
        // Index 4: args_get
        imports.import(
            "wasi_snapshot_preview1",
            "args_get",
            wasm_encoder::EntityType::Function(WASI_RANDOM_GET_TYPE_IDX), // (i32,i32)->i32
        );
        // Index 5: args_sizes_get
        imports.import(
            "wasi_snapshot_preview1",
            "args_sizes_get",
            wasm_encoder::EntityType::Function(WASI_RANDOM_GET_TYPE_IDX),
        );
        // Index 6: environ_get
        imports.import(
            "wasi_snapshot_preview1",
            "environ_get",
            wasm_encoder::EntityType::Function(WASI_RANDOM_GET_TYPE_IDX),
        );
        // Index 7: environ_sizes_get
        imports.import(
            "wasi_snapshot_preview1",
            "environ_sizes_get",
            wasm_encoder::EntityType::Function(WASI_RANDOM_GET_TYPE_IDX),
        );
        // Index 8: proc_exit
        imports.import(
            "wasi_snapshot_preview1",
            "proc_exit",
            wasm_encoder::EntityType::Function(WASI_PROC_EXIT_TYPE_IDX),
        );
        // Index 9: path_open
        imports.import(
            "wasi_snapshot_preview1",
            "path_open",
            wasm_encoder::EntityType::Function(WASI_PATH_OPEN_TYPE_IDX),
        );
        // Index 10: fd_close
        imports.import(
            "wasi_snapshot_preview1",
            "fd_close",
            wasm_encoder::EntityType::Function(WASI_FD_CLOSE_TYPE_IDX),
        );

        // --- Ark Host Imports (index 11..24) ---
        // Math unary: sin, cos, tan, asin, acos, atan, sqrt  (type 6: i64 -> i64)
        for name in &[
            "math_sin",
            "math_cos",
            "math_tan",
            "math_asin",
            "math_acos",
            "math_atan",
        ] {
            imports.import(
                "ark_host",
                name,
                wasm_encoder::EntityType::Function(ARK_HOST_UNARY_I64_TYPE_IDX),
            );
        }
        // Index 17: math_atan2  (type 7: i64,i64 -> i64)
        imports.import(
            "ark_host",
            "math_atan2",
            wasm_encoder::EntityType::Function(ARK_HOST_BINARY_I64_TYPE_IDX),
        );
        // Index 18: math_sqrt  (type 6: i64 -> i64)
        imports.import(
            "ark_host",
            "math_sqrt",
            wasm_encoder::EntityType::Function(ARK_HOST_UNARY_I64_TYPE_IDX),
        );
        // Index 19: math_pow  (type 7: i64,i64 -> i64)
        imports.import(
            "ark_host",
            "math_pow",
            wasm_encoder::EntityType::Function(ARK_HOST_BINARY_I64_TYPE_IDX),
        );
        // Index 20: math_pow_mod  (type 8: i64,i64,i64 -> i64)
        imports.import(
            "ark_host",
            "math_pow_mod",
            wasm_encoder::EntityType::Function(ARK_HOST_TERNARY_I64_TYPE_IDX),
        );
        // Index 21: crypto_sha512  (type 9: i32,i32,i32 -> i32)
        imports.import(
            "ark_host",
            "crypto_sha512",
            wasm_encoder::EntityType::Function(ARK_HOST_MEM_3I32_TYPE_IDX),
        );
        // Index 22: json_parse  (type 9: i32,i32,i32 -> i32)
        imports.import(
            "ark_host",
            "json_parse",
            wasm_encoder::EntityType::Function(ARK_HOST_MEM_3I32_TYPE_IDX),
        );
        // Index 23: json_stringify  (type 9: i32,i32,i32 -> i32)
        imports.import(
            "ark_host",
            "json_stringify",
            wasm_encoder::EntityType::Function(ARK_HOST_MEM_3I32_TYPE_IDX),
        );
        // Index 24: ask_ai  (type 10: i32,i32,i32,i32 -> i32)
        imports.import(
            "ark_host",
            "ask_ai",
            wasm_encoder::EntityType::Function(ARK_HOST_MEM_4I32_TYPE_IDX),
        );

        module.section(&imports);

        // --- Function Section ---
        let mut functions = FunctionSection::new();
        for (type_idx, _, _) in &self.functions {
            functions.function(*type_idx);
        }
        module.section(&functions);

        // --- Table Section (for call_indirect / lambda dispatch) ---
        let total_funcs = self.import_count + self.functions.len() as u32;
        if total_funcs > 0 {
            let mut tables = TableSection::new();
            tables.table(TableType {
                element_type: wasm_encoder::RefType::FUNCREF,
                minimum: total_funcs as u64,
                maximum: Some(total_funcs as u64),
                table64: false,
                shared: false,
            });
            module.section(&tables);
        }

        // --- Memory Section ---
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 1, // 1 page = 64KB
            maximum: Some(16),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memories);

        // --- Compute heap start (after all string data) ---
        let mut max_string_end = STRING_MEMORY_START;
        for (_, _, ctx) in &self.functions {
            for (offset, bytes) in &ctx.string_data {
                let end = *offset + bytes.len() as i32;
                if end > max_string_end {
                    max_string_end = end;
                }
            }
        }
        // Align to 8 bytes and add safety padding
        let heap_start = ((max_string_end + 7) & !7).max(2048);

        // --- Global Section (__heap_ptr) ---
        let mut globals = GlobalSection::new();
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(heap_start),
        );
        module.section(&globals);

        // --- Export Section ---
        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        exports.export("__heap_ptr", ExportKind::Global, 0);

        // Backward compatibility: if NO function has #[export], export everything
        let has_any_export_attrs = self
            .func_attributes
            .values()
            .any(|attrs| attrs.iter().any(|a| a == "export"));

        for (i, (_, name, _)) in self.functions.iter().enumerate() {
            let func_idx = self.import_count + i as u32;
            if has_any_export_attrs {
                // Selective mode: only export #[export]-marked or system functions
                if self.should_export(name) {
                    exports.export(name, ExportKind::Func, func_idx);
                }
            } else {
                // Legacy mode: export everything (backward compat)
                exports.export(name, ExportKind::Func, func_idx);
            }
        }
        module.section(&exports);

        // --- Element Section (populate table with function references) ---
        let total_funcs = self.import_count + self.functions.len() as u32;
        if total_funcs > 0 {
            let mut elements = ElementSection::new();
            // Active element segment: fill table 0 starting at offset 0
            // with all function indices (imports + user functions)
            let func_indices: Vec<u32> = (0..total_funcs).collect();
            elements.active(
                Some(0), // table index
                &wasm_encoder::ConstExpr::i32_const(0),
                wasm_encoder::Elements::Functions(std::borrow::Cow::Borrowed(&func_indices)),
            );
            module.section(&elements);
        }

        // --- Code Section ---
        let mut codes = CodeSection::new();
        for (_, _, ctx) in &self.functions {
            let extra_locals = ctx.scope.extra_local_count(ctx.param_count);
            let locals: Vec<(u32, ValType)> = if extra_locals > 0 {
                vec![(extra_locals, ValType::I64)]
            } else {
                vec![]
            };
            let mut func = Function::new(locals);
            for instr in &ctx.instructions {
                func.instruction(instr);
            }
            codes.function(&func);
        }
        module.section(&codes);

        // --- Data Section (string constants) ---
        if !self.data_segments.is_empty()
            || self
                .functions
                .iter()
                .any(|(_, _, ctx)| !ctx.string_data.is_empty())
        {
            let mut data = wasm_encoder::DataSection::new();
            // Global data segments
            for (offset, bytes) in &self.data_segments {
                data.active(
                    0,
                    &wasm_encoder::ConstExpr::i32_const(*offset),
                    bytes.clone(),
                );
            }
            // Per-function string data
            for (_, _, ctx) in &self.functions {
                for (offset, bytes) in &ctx.string_data {
                    data.active(
                        0,
                        &wasm_encoder::ConstExpr::i32_const(*offset),
                        bytes.clone(),
                    );
                }
            }
            module.section(&data);
        }

        module.finish()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ArkNode, Expression, FunctionDef, MastNode, Statement};
    use crate::types::ArkType;

    /// Helper: wrap an ArkNode into a MastNode
    fn mast(node: ArkNode) -> MastNode {
        MastNode::new(node).unwrap()
    }

    #[test]
    fn test_compile_integer_literal() {
        let ast = ArkNode::Statement(Statement::Block(vec![Statement::Return(
            Expression::Integer(42),
        )]));

        let func_def = FunctionDef {
            name: "main".to_string(),
            inputs: vec![],
            output: ArkType::Integer,
            body: Box::new(mast(ast)),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func_def)]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
        let wasm = result.unwrap();
        // Valid WASM starts with magic number \0asm
        assert_eq!(&wasm[0..4], b"\0asm", "Invalid WASM magic number");
        // Version 1
        assert_eq!(wasm[4], 1, "Expected WASM version 1");
    }

    #[test]
    fn test_compile_arithmetic() {
        // add(2, 3) => should compile to i64.const 2, i64.const 3, i64.add
        let ast = ArkNode::Statement(Statement::Block(vec![Statement::Return(
            Expression::Call {
                function_hash: "add".to_string(),
                args: vec![Expression::Integer(2), Expression::Integer(3)],
            },
        )]));

        let func_def = FunctionDef {
            name: "test_add".to_string(),
            inputs: vec![],
            output: ArkType::Integer,
            body: Box::new(mast(ast)),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func_def)]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
        let wasm = result.unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
    }

    #[test]
    fn test_compile_if_else() {
        // if (eq(1, 1)) { 42 } else { 0 }
        let ast = ArkNode::Statement(Statement::Block(vec![Statement::Return(
            Expression::Integer(99), // simplified — full if/else tested via Statement::If
        )]));

        let if_stmt = Statement::If {
            condition: Expression::Call {
                function_hash: "eq".to_string(),
                args: vec![Expression::Integer(1), Expression::Integer(1)],
            },
            then_block: vec![Statement::Return(Expression::Integer(42))],
            else_block: Some(vec![Statement::Return(Expression::Integer(0))]),
        };

        let func_def = FunctionDef {
            name: "test_if".to_string(),
            inputs: vec![],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Statement(Statement::Block(vec![
                if_stmt,
                Statement::Return(Expression::Integer(99)),
            ])))),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func_def)]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_function_with_params() {
        // fn add_two(a: Int, b: Int) -> Int { add(a, b) }
        let func_def = FunctionDef {
            name: "add_two".to_string(),
            inputs: vec![
                ("a".to_string(), ArkType::Integer),
                ("b".to_string(), ArkType::Integer),
            ],
            output: ArkType::Integer,
            body: Box::new(mast(ArkNode::Statement(Statement::Return(
                Expression::Call {
                    function_hash: "add".to_string(),
                    args: vec![
                        Expression::Variable("a".to_string()),
                        Expression::Variable("b".to_string()),
                    ],
                },
            )))),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func_def)]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_while_loop() {
        // let x = 0; while (lt(x, 10)) { x = add(x, 1) }; return x
        let body = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "x".to_string(),
                ty: None,
                value: Expression::Integer(0),
            },
            Statement::While {
                condition: Expression::Call {
                    function_hash: "lt".to_string(),
                    args: vec![
                        Expression::Variable("x".to_string()),
                        Expression::Integer(10),
                    ],
                },
                body: vec![Statement::Let {
                    name: "x".to_string(),
                    ty: None,
                    value: Expression::Call {
                        function_hash: "add".to_string(),
                        args: vec![
                            Expression::Variable("x".to_string()),
                            Expression::Integer(1),
                        ],
                    },
                }],
            },
            Statement::Return(Expression::Variable("x".to_string())),
        ]));

        let func_def = FunctionDef {
            name: "loop_test".to_string(),
            inputs: vec![],
            output: ArkType::Integer,
            body: Box::new(mast(body)),
            attributes: vec![],
        };

        let program = ArkNode::Statement(Statement::Block(vec![Statement::Function(func_def)]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_start_function() {
        // Top-level: print(42)
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "print".to_string(),
                args: vec![Expression::Integer(42)],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
        let wasm = result.unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
        assert!(
            wasm.len() > 8,
            "WASM binary too small: {} bytes",
            wasm.len()
        );
    }

    #[test]
    fn test_compile_string_literal() {
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Literal("hello world".to_string()),
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_let_and_variable() {
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "x".to_string(),
                ty: None,
                value: Expression::Integer(100),
            },
            Statement::Expression(Expression::Variable("x".to_string())),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_nested_calls() {
        // add(mul(2, 3), sub(10, 4)) = 6 + 6 = 12
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "add".to_string(),
                args: vec![
                    Expression::Call {
                        function_hash: "mul".to_string(),
                        args: vec![Expression::Integer(2), Expression::Integer(3)],
                    },
                    Expression::Call {
                        function_hash: "sub".to_string(),
                        args: vec![Expression::Integer(10), Expression::Integer(4)],
                    },
                ],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Compilation failed: {:?}", result.err());
    }

    #[test]
    fn test_wasm_magic_and_version() {
        let program = ArkNode::Statement(Statement::Block(vec![]));
        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok());
        let wasm = result.unwrap();
        assert!(wasm.len() >= 8);
        // Magic: \0asm
        assert_eq!(wasm[0], 0x00);
        assert_eq!(wasm[1], 0x61); // 'a'
        assert_eq!(wasm[2], 0x73); // 's'
        assert_eq!(wasm[3], 0x6d); // 'm'
        // Version: 1
        assert_eq!(wasm[4], 0x01);
        assert_eq!(wasm[5], 0x00);
        assert_eq!(wasm[6], 0x00);
        assert_eq!(wasm[7], 0x00);
    }

    #[test]
    fn test_compile_print_string() {
        // Top-level: print("hello")
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "print".to_string(),
                args: vec![Expression::Literal("hello".to_string())],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "String print compilation failed: {:?}",
            result.err()
        );
        let wasm = result.unwrap();
        assert_eq!(&wasm[0..4], b"\0asm");
        // String data should be embedded in the binary
        assert!(
            wasm.len() > 50,
            "WASM binary too small for string data: {} bytes",
            wasm.len()
        );
    }

    #[test]
    fn test_compile_print_mixed() {
        // Top-level: print("hello"); print(42)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Expression(Expression::Call {
                function_hash: "print".to_string(),
                args: vec![Expression::Literal("hello world".to_string())],
            }),
            Statement::Expression(Expression::Call {
                function_hash: "print".to_string(),
                args: vec![Expression::Integer(42)],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "Mixed print compilation failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_compile_list_literal() {
        // let xs = [1, 2, 3]
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Let {
            name: "xs".to_string(),
            ty: None,
            value: Expression::List(vec![
                Expression::Integer(1),
                Expression::Integer(2),
                Expression::Integer(3),
            ]),
        }]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "List compilation failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_compile_struct_init() {
        // let p = { x: 10, y: 20 }
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Let {
            name: "p".to_string(),
            ty: None,
            value: Expression::StructInit {
                fields: vec![
                    ("x".to_string(), Expression::Integer(10)),
                    ("y".to_string(), Expression::Integer(20)),
                ],
            },
        }]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "Struct init failed: {:?}", result.err());
    }

    #[test]
    fn test_compile_get_field_length() {
        // [1, 2, 3].length
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::GetField {
                obj: Box::new(Expression::List(vec![
                    Expression::Integer(1),
                    Expression::Integer(2),
                    Expression::Integer(3),
                ])),
                field: "length".to_string(),
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "GetField .length failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_compile_match_expression() {
        // match 1 { 1 => 10, 2 => 20, _ => 0 }
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Match {
                scrutinee: Box::new(Expression::Integer(1)),
                arms: vec![
                    (Pattern::Literal("1".to_string()), Expression::Integer(10)),
                    (Pattern::Literal("2".to_string()), Expression::Integer(20)),
                    (Pattern::Wildcard, Expression::Integer(0)),
                ],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "Match compilation failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_compile_let_destructure() {
        // let [a, b] = [10, 20]
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::LetDestructure {
                names: vec!["a".to_string(), "b".to_string()],
                value: Expression::List(vec![Expression::Integer(10), Expression::Integer(20)]),
            },
            Statement::Expression(Expression::Variable("a".to_string())),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "LetDestructure failed: {:?}", result.err());
    }

    // =========================================================================
    // Tier 1 Intrinsic Tests
    // =========================================================================

    #[test]
    fn test_intrinsic_len() {
        // len([1, 2, 3]) => 3
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "len".to_string(),
                args: vec![Expression::List(vec![
                    Expression::Integer(1),
                    Expression::Integer(2),
                    Expression::Integer(3),
                ])],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "len intrinsic failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_list_get() {
        // let xs = [10, 20, 30]; list.get(xs, 1) => 20
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "xs".to_string(),
                ty: None,
                value: Expression::List(vec![
                    Expression::Integer(10),
                    Expression::Integer(20),
                    Expression::Integer(30),
                ]),
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_list_get".to_string(),
                args: vec![
                    Expression::Variable("xs".to_string()),
                    Expression::Integer(1),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "list.get failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_list_set() {
        // let xs = [10, 20]; list.set(xs, 0, 99)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "xs".to_string(),
                ty: None,
                value: Expression::List(vec![Expression::Integer(10), Expression::Integer(20)]),
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_list_set".to_string(),
                args: vec![
                    Expression::Variable("xs".to_string()),
                    Expression::Integer(0),
                    Expression::Integer(99),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "list.set failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_list_append() {
        // let xs = [1, 2]; list.append(xs, 3)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "xs".to_string(),
                ty: None,
                value: Expression::List(vec![Expression::Integer(1), Expression::Integer(2)]),
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_list_append".to_string(),
                args: vec![
                    Expression::Variable("xs".to_string()),
                    Expression::Integer(3),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "list.append failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_list_pop() {
        // let xs = [10, 20, 30]; list.pop(xs)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "xs".to_string(),
                ty: None,
                value: Expression::List(vec![
                    Expression::Integer(10),
                    Expression::Integer(20),
                    Expression::Integer(30),
                ]),
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_list_pop".to_string(),
                args: vec![Expression::Variable("xs".to_string())],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "list.pop failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_list_delete() {
        // let xs = [10, 20, 30]; list.delete(xs, 1)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "xs".to_string(),
                ty: None,
                value: Expression::List(vec![
                    Expression::Integer(10),
                    Expression::Integer(20),
                    Expression::Integer(30),
                ]),
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_list_delete".to_string(),
                args: vec![
                    Expression::Variable("xs".to_string()),
                    Expression::Integer(1),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "list.delete failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_struct_get() {
        // let s = { x: 10, y: 20 }; struct.get(s, 0) => 10
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "s".to_string(),
                ty: None,
                value: Expression::StructInit {
                    fields: vec![
                        ("x".to_string(), Expression::Integer(10)),
                        ("y".to_string(), Expression::Integer(20)),
                    ],
                },
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_struct_get".to_string(),
                args: vec![
                    Expression::Variable("s".to_string()),
                    Expression::Integer(0),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "struct.get failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_struct_set() {
        // let s = { x: 10 }; struct.set(s, 0, 99)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "s".to_string(),
                ty: None,
                value: Expression::StructInit {
                    fields: vec![("x".to_string(), Expression::Integer(10))],
                },
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_struct_set".to_string(),
                args: vec![
                    Expression::Variable("s".to_string()),
                    Expression::Integer(0),
                    Expression::Integer(99),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "struct.set failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_struct_has() {
        // let s = { x: 10, y: 20 }; struct.has(s, 1) => true (1)
        let program = ArkNode::Statement(Statement::Block(vec![
            Statement::Let {
                name: "s".to_string(),
                ty: None,
                value: Expression::StructInit {
                    fields: vec![
                        ("x".to_string(), Expression::Integer(10)),
                        ("y".to_string(), Expression::Integer(20)),
                    ],
                },
            },
            Statement::Expression(Expression::Call {
                function_hash: "intrinsic_struct_has".to_string(),
                args: vec![
                    Expression::Variable("s".to_string()),
                    Expression::Integer(1),
                ],
            }),
        ]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "struct.has failed: {:?}", result.err());
    }

    // =========================================================================
    // Phase 11: WASI Intrinsic Tests
    // =========================================================================

    #[test]
    fn test_intrinsic_time_now() {
        // sys.time.now() → nanosecond timestamp
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "sys.time.now".to_string(),
                args: vec![],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "sys.time.now compile failed: {:?}",
            result.err()
        );

        // Verify the module can be loaded and run via wasmtime with WASI stubs
        let bytes = result.unwrap();
        let run_result = crate::wasm_runner::run_wasm(&bytes);
        assert!(
            run_result.is_ok(),
            "sys.time.now execution failed: {:?}",
            run_result.err()
        );
    }

    #[test]
    fn test_intrinsic_exit() {
        // sys.exit(0) → proc_exit
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "sys.exit".to_string(),
                args: vec![Expression::Integer(0)],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "sys.exit compile failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_intrinsic_exit_no_args() {
        // exit() → proc_exit with default code 0
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "exit".to_string(),
                args: vec![],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "exit() compile failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_random_bytes() {
        // sys.crypto.random_bytes(32) → pointer to 32 random bytes
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "sys.crypto.random_bytes".to_string(),
                args: vec![Expression::Integer(32)],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "sys.crypto.random_bytes failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_intrinsic_fd_close() {
        // fd_close(3) → close fd 3
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "fd_close".to_string(),
                args: vec![Expression::Integer(3)],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "fd_close failed: {:?}", result.err());
    }

    #[test]
    fn test_intrinsic_io_read_line() {
        // sys.io.read_line() → bytes read from stdin
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "sys.io.read_line".to_string(),
                args: vec![],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "sys.io.read_line failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_wasi_module_validates() {
        // Verify that a module with ALL WASI imports validates correctly
        let source = "print(42)";
        let ast = crate::parser::parse_source(source, "test.ark").expect("parse failed");
        let bytes = WasmCodegen::compile_to_bytes(&ast).expect("compile failed");

        // Module should validate with wasmparser
        let result = wasmparser::Validator::new().validate_all(&bytes);
        assert!(
            result.is_ok(),
            "WASI module validation failed: {:?}",
            result.err()
        );
    }

    // =========================================================================
    // Phase 14: String Operations Tests
    // =========================================================================

    #[test]
    fn test_string_len_intrinsic() {
        // string_len("hello") → should compile and validate
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "string_len".to_string(),
                args: vec![Expression::Literal("hello".to_string())],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "string_len failed: {:?}", result.err());
        let bytes = result.unwrap();
        let valid = wasmparser::Validator::new().validate_all(&bytes);
        assert!(
            valid.is_ok(),
            "string_len WASM validation failed: {:?}",
            valid.err()
        );
    }

    #[test]
    fn test_string_concat_intrinsic() {
        // string_concat("hello", " world") → should compile and validate
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "string_concat".to_string(),
                args: vec![
                    Expression::Literal("hello".to_string()),
                    Expression::Literal(" world".to_string()),
                ],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "string_concat failed: {:?}", result.err());
        let bytes = result.unwrap();
        let valid = wasmparser::Validator::new().validate_all(&bytes);
        assert!(
            valid.is_ok(),
            "string_concat WASM validation failed: {:?}",
            valid.err()
        );
    }

    #[test]
    fn test_string_eq_intrinsic() {
        // string_eq("hello", "hello") → should compile and validate
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "string_eq".to_string(),
                args: vec![
                    Expression::Literal("hello".to_string()),
                    Expression::Literal("hello".to_string()),
                ],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "string_eq failed: {:?}", result.err());
        let bytes = result.unwrap();
        let valid = wasmparser::Validator::new().validate_all(&bytes);
        assert!(
            valid.is_ok(),
            "string_eq WASM validation failed: {:?}",
            valid.err()
        );
    }

    #[test]
    fn test_string_slice_intrinsic() {
        // string_slice("hello world", 0, 5) → should compile and validate
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "string_slice".to_string(),
                args: vec![
                    Expression::Literal("hello world".to_string()),
                    Expression::Integer(0),
                    Expression::Integer(5),
                ],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_ok(), "string_slice failed: {:?}", result.err());
        let bytes = result.unwrap();
        let valid = wasmparser::Validator::new().validate_all(&bytes);
        assert!(
            valid.is_ok(),
            "string_slice WASM validation failed: {:?}",
            valid.err()
        );
    }

    #[test]
    fn test_string_eq_arity_error() {
        // string_eq with wrong arity should fail
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Call {
                function_hash: "string_eq".to_string(),
                args: vec![Expression::Literal("only_one".to_string())],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(result.is_err(), "string_eq with 1 arg should fail");
    }

    // =========================================================================
    // Phase 14: Lambda / Closure Tests
    // =========================================================================

    #[test]
    fn test_lambda_expression_compiles() {
        // Lambda expression with inline body: func(x) { x + 1 }
        let program = ArkNode::Statement(Statement::Block(vec![Statement::Expression(
            Expression::Lambda {
                params: vec!["x".to_string()],
                body: vec![Statement::Expression(Expression::Call {
                    function_hash: "intrinsic_add".to_string(),
                    args: vec![
                        Expression::Variable("x".to_string()),
                        Expression::Integer(1),
                    ],
                })],
            },
        )]));

        let result = WasmCodegen::compile_to_bytes(&program);
        assert!(
            result.is_ok(),
            "Lambda compilation failed: {:?}",
            result.err()
        );
        let bytes = result.unwrap();
        let valid = wasmparser::Validator::new().validate_all(&bytes);
        assert!(
            valid.is_ok(),
            "Lambda WASM validation failed: {:?}",
            valid.err()
        );
    }
}
