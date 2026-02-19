use crate::bytecode::{Chunk, OpCode};
use crate::debugger::DebugAction;
use crate::intrinsics;
use crate::runtime::{RuntimeError, Scope, Value};
use std::sync::Arc;
use thiserror::Error;

pub const MAX_STACK_DEPTH: usize = 10_000;
pub const MAX_STEPS: u64 = 10_000_000;

#[derive(Error, Debug)]
pub enum ArkError {
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Stack underflow executing {0}")]
    StackUnderflow(String),
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Execution timeout")]
    ExecutionTimeout,
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("VM Error: {0}")]
    Generic(String),
}

// --- GraphArena Implementation ---

#[derive(Debug)]
pub struct GraphArena {
    pub nodes: Vec<Option<GraphNode>>,
    pub free_list: Vec<usize>,
}

impl Default for GraphArena {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphArena {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn alloc(&mut self, data: GraphData) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.nodes[idx] = Some(GraphNode { data, ref_count: 1 });
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(GraphNode { data, ref_count: 1 }));
            idx
        }
    }

    pub fn get(&self, idx: usize) -> Option<&GraphNode> {
        self.nodes.get(idx).and_then(|opt| opt.as_ref())
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut GraphNode> {
        self.nodes.get_mut(idx).and_then(|opt| opt.as_mut())
    }

    pub fn decref(&mut self, idx: usize) {
        let should_free = if let Some(node) = self.get_mut(idx) {
            node.ref_count = node.ref_count.saturating_sub(1);
            node.ref_count == 0
        } else {
            false
        };

        if should_free {
            self.nodes[idx] = None;
            self.free_list.push(idx);
        }
    }

    pub fn incref(&mut self, idx: usize) {
        if let Some(node) = self.get_mut(idx) {
            node.ref_count += 1;
        }
    }
}

#[derive(Debug)]
pub struct GraphNode {
    pub data: GraphData,
    pub ref_count: usize,
}

#[derive(Debug)]
pub enum GraphData {
    Frame(CallFrame),
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub ip: usize,
    pub chunk: Arc<Chunk>,
}

/// Type alias for VM debug hook callback.
pub type DebugHookFn<'a> = dyn FnMut(&[Value], &[Scope], usize, &Chunk) -> DebugAction + 'a;

pub struct VM<'a> {
    pub heap: GraphArena,
    pub stack: Vec<Value>,
    pub scopes: Vec<Scope<'a>>, // Stack of scopes (Frames)
    pub frames: Vec<usize>,     // Stack of frame indices into heap
    pub ip: usize,
    pub chunk: Arc<Chunk>,
    pub security_level: u8,
    pub step_count: u64,
    pub trace: bool,
    /// Optional debug hook: called before each opcode, receives (ip). Returns DebugAction.
    pub debug_hook: Option<Box<DebugHookFn<'a>>>,
}

impl<'a> VM<'a> {
    pub fn new(chunk: Chunk, hash: &str, security_level: u8) -> Result<Self, RuntimeError> {
        // Security Check: Verify Code Hash on Blockchain
        if security_level > 0 && !crate::blockchain::verify_code_hash(hash) {
            return Err(RuntimeError::UntrustedCode);
        }

        let mut global_scope = Scope::new();
        crate::intrinsics::IntrinsicRegistry::register_all(&mut global_scope);
        Ok(Self {
            heap: GraphArena::new(),
            stack: Vec::new(),
            scopes: vec![global_scope],
            frames: Vec::new(),
            ip: 0,
            chunk: Arc::new(chunk),
            security_level,
            step_count: 0,
            trace: false,
            debug_hook: None,
        })
    }

    #[inline]
    pub fn push(&mut self, val: Value) -> Result<(), ArkError> {
        if self.stack.len() >= MAX_STACK_DEPTH {
            return Err(ArkError::StackOverflow);
        }
        self.stack.push(val);
        Ok(())
    }

    /// Calls a public function by name with the given arguments.
    /// This is used by the host environment (e.g., WASM, FFI) to invoke specific Ark functions.
    pub fn call_public_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, ArkError> {
        // 1. Find the function
        let func = self
            .find_var(name)
            .ok_or_else(|| ArkError::Generic(format!("Function '{}' not found", name)))?;

        // 2. Push Arguments
        let arg_count = args.len();
        for arg in args {
            self.push(arg)?;
        }

        // 3. Push Function
        self.push(func)?;

        // 4. Trigger Call (sets up frame, ip, chunk)
        // Note: op_call pops the function, then the arguments.
        // So Stack must be: [Arg1, Arg2, ... ArgN, Func]
        // This matches our push order above.
        // op_call(arg_count) expects `arg_count` arguments on the stack below the function.
        self.op_call(arg_count)?;

        // 5. Run the VM until return
        self.run()
    }

    pub fn run(&mut self) -> Result<Value, ArkError> {
        loop {
            // Execution Timeout Check
            self.step_count += 1;
            if self.step_count > MAX_STEPS {
                return Err(ArkError::ExecutionTimeout);
            }

            if self.ip >= self.chunk.code.len() {
                if let Some(val) = self.op_return()? {
                    return Ok(val);
                }
                continue;
            }

            let op = &self.chunk.code[self.ip];

            // Debug hook — fire before execution
            if self.debug_hook.is_some() {
                // We need to temporarily take the hook to avoid borrow issues
                let mut hook = self.debug_hook.take().unwrap();
                let action = hook(&self.stack, &self.scopes, self.ip, &self.chunk);
                self.debug_hook = Some(hook);
                match action {
                    DebugAction::Quit => return Ok(Value::Unit),
                    DebugAction::Continue => {}
                }
            }

            if self.trace {
                println!("IP: {:03} | Op: {:?}", self.ip, op);
            }

            #[cfg(debug_assertions)]
            {
                use std::io::Write;
                println!(
                    "IP: {:03} | Op: {:?} | Stack: {} | Frames: {}",
                    self.ip,
                    op,
                    self.stack.len(),
                    self.frames.len()
                );
                std::io::stdout().flush().unwrap();
            }
            self.ip += 1;

            match op {
                OpCode::Push(v) => self.push(v.clone())?,
                OpCode::Pop => {
                    self.stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("Pop".to_string()))?;
                }

                OpCode::Add => self.binary_op("Add", |a, b| {
                    intrinsics::intrinsic_add(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Sub => self.binary_op("Sub", |a, b| {
                    intrinsics::intrinsic_sub(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Mul => self.binary_op("Mul", |a, b| {
                    intrinsics::intrinsic_mul(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Div => self.binary_op("Div", |a, b| match b {
                    Value::Integer(0) => Err(ArkError::DivisionByZero),
                    _ => intrinsics::intrinsic_div(vec![a, b]).map_err(ArkError::from),
                })?,
                OpCode::Mod => self.binary_op("Mod", |a, b| match b {
                    Value::Integer(0) => Err(ArkError::DivisionByZero),
                    _ => intrinsics::intrinsic_mod(vec![a, b]).map_err(ArkError::from),
                })?,

                OpCode::Eq => self.binary_op("Eq", |a, b| {
                    intrinsics::intrinsic_eq(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Neq => self.binary_op_manual("Neq", |a, b| {
                    let res =
                        intrinsics::intrinsic_eq(vec![a, b]).map_err(|e| format!("{:?}", e))?;
                    if let Value::Boolean(b) = res {
                        Ok(Value::Boolean(!b))
                    } else {
                        Err("Eq did not return boolean".to_string())
                    }
                })?,
                OpCode::Gt => self.binary_op("Gt", |a, b| {
                    intrinsics::intrinsic_gt(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Lt => self.binary_op("Lt", |a, b| {
                    intrinsics::intrinsic_lt(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Ge => self.binary_op("Ge", |a, b| {
                    intrinsics::intrinsic_ge(vec![a, b]).map_err(ArkError::from)
                })?,
                OpCode::Le => self.binary_op("Le", |a, b| {
                    intrinsics::intrinsic_le(vec![a, b]).map_err(ArkError::from)
                })?,

                OpCode::And => {
                    let b = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("And".to_string()))?;
                    let a = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("And".to_string()))?;
                    let res = intrinsics::intrinsic_and(vec![a, b])?;
                    self.push(res)?;
                }
                OpCode::Or => {
                    let b = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("Or".to_string()))?;
                    let a = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("Or".to_string()))?;
                    let res = intrinsics::intrinsic_or(vec![a, b])?;
                    self.push(res)?;
                }
                OpCode::Not => {
                    let a = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("Not".to_string()))?;
                    let res = intrinsics::intrinsic_not(vec![a])?;
                    self.push(res)?;
                }

                OpCode::Print => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("Print".to_string()))?;
                    #[cfg(not(test))]
                    println!("{:?}", val);
                    #[cfg(test)]
                    let _ = val;
                }
                OpCode::Destructure => self.op_destructure()?,

                OpCode::MakeList(size) => self.op_make_list(*size)?,

                OpCode::MakeStruct(size) => self.op_make_struct(*size)?,

                OpCode::GetField(field) => {
                    let obj = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("GetField".to_string()))?;
                    if let Value::Struct(mut fields) = obj {
                        if let Some(val) = fields.remove(field) {
                            self.push(val)?;
                        } else {
                            return Err(ArkError::Generic(format!(
                                "Field '{}' not found in struct",
                                field
                            )));
                        }
                    } else {
                        return Err(ArkError::Generic(format!(
                            "GetField expected Struct, got {:?}",
                            obj
                        )));
                    }
                }

                OpCode::SetField(field) => {
                    let obj = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("SetField".to_string()))?;
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("SetField".to_string()))?;

                    if let Value::Struct(mut fields) = obj {
                        fields.insert(field.clone(), val);
                        self.push(Value::Struct(fields))?;
                    } else {
                        return Err(ArkError::Generic(format!(
                            "SetField expected Struct, got {:?}",
                            obj
                        )));
                    }
                }

                OpCode::Load(name) => {
                    let val = self.find_var(name).ok_or_else(|| {
                        ArkError::Generic(format!("Variable not found: {}", name))
                    })?;
                    self.push(val)?;
                }
                OpCode::Store(name) => {
                    let val = self
                        .stack
                        .last()
                        .ok_or_else(|| ArkError::StackUnderflow("Store".to_string()))?
                        .clone();
                    // Store in current scope
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.set(name.clone(), val);
                    }
                    self.stack.pop(); // Store consumes value? Let's say yes for now.
                }

                OpCode::Jmp(offset) => {
                    self.ip = *offset;
                }
                OpCode::JmpIfFalse(offset) => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| ArkError::StackUnderflow("JmpIfFalse".to_string()))?;
                    let is_true = match val {
                        Value::Boolean(b) => b,
                        Value::Integer(i) => i != 0,
                        // String truthiness consistency
                        Value::String(s) => !s.is_empty(),
                        _ => false,
                    };
                    if !is_true {
                        self.ip = *offset;
                    }
                }

                OpCode::Call(arg_count) => self.op_call(*arg_count)?,

                OpCode::Ret => {
                    if let Some(val) = self.op_return()? {
                        return Ok(val);
                    }
                }

                OpCode::MakeEnum(enum_name, variant, field_count) => {
                    let mut fields = Vec::with_capacity(*field_count);
                    for _ in 0..*field_count {
                        let val = self
                            .stack
                            .pop()
                            .ok_or_else(|| ArkError::StackUnderflow("MakeEnum".to_string()))?;
                        fields.push(val);
                    }
                    fields.reverse(); // Fields were pushed left-to-right
                    self.push(Value::EnumValue {
                        enum_name: enum_name.clone(),
                        variant: variant.clone(),
                        fields,
                    })?;
                }
            }
        }
    }

    fn binary_op<F>(&mut self, name: &str, op_fn: F) -> Result<(), ArkError>
    where
        F: Fn(Value, Value) -> Result<Value, ArkError>,
    {
        let b = self
            .stack
            .pop()
            .ok_or_else(|| ArkError::StackUnderflow(name.to_string()))?;
        let a = self
            .stack
            .pop()
            .ok_or_else(|| ArkError::StackUnderflow(name.to_string()))?;
        let res = op_fn(a, b)?;
        self.push(res)
    }

    fn binary_op_manual<F>(&mut self, name: &str, op_fn: F) -> Result<(), ArkError>
    where
        F: Fn(Value, Value) -> Result<Value, String>,
    {
        let b = self
            .stack
            .pop()
            .ok_or_else(|| ArkError::StackUnderflow(name.to_string()))?;
        let a = self
            .stack
            .pop()
            .ok_or_else(|| ArkError::StackUnderflow(name.to_string()))?;
        let res = op_fn(a, b).map_err(ArkError::Generic)?;
        self.push(res)
    }

    #[inline]
    fn op_return(&mut self) -> Result<Option<Value>, ArkError> {
        let result = self.stack.pop().unwrap_or(Value::Unit);

        if let Some(frame_idx) = self.frames.pop() {
            // Restore Context from Heap
            let (new_chunk, new_ip) = if let Some(node) = self.heap.get(frame_idx) {
                let GraphData::Frame(ref frame) = node.data;
                (frame.chunk.clone(), frame.ip)
            } else {
                return Err(ArkError::Generic(format!(
                    "Frame not found at index {}",
                    frame_idx
                )));
            };

            self.chunk = new_chunk;
            self.ip = new_ip;

            // Decrement ref count (and free if 0)
            self.heap.decref(frame_idx);

            self.scopes.pop();
            self.push(result)?;
            Ok(None) // Continue loop
        } else {
            // Return from main
            Ok(Some(result))
        }
    }

    #[inline]
    fn op_call(&mut self, arg_count: usize) -> Result<(), ArkError> {
        let func_val = self
            .stack
            .pop()
            .ok_or_else(|| ArkError::StackUnderflow("Call".to_string()))?;
        match func_val {
            Value::Function(chunk) => {
                // Allocation on Heap (Zero-Copy Ref Count)
                let frame = CallFrame {
                    ip: self.ip,
                    chunk: self.chunk.clone(),
                };
                let frame_idx = self.heap.alloc(GraphData::Frame(frame));
                self.frames.push(frame_idx);

                // Switch Context
                self.chunk = chunk;
                self.ip = 0;
                // Push Scope
                self.scopes.push(Scope::new());
                Ok(())
            }
            Value::NativeFunction(func) => {
                let mut args = Vec::new();
                for _ in 0..arg_count {
                    args.push(
                        self.stack
                            .pop()
                            .ok_or_else(|| ArkError::StackUnderflow("NativeCall".to_string()))?,
                    );
                }
                // Arguments popped in reverse order (last arg first)
                // We need them in forward order [arg1, arg2]
                args.reverse();

                let result = func(args).map_err(ArkError::from)?;
                self.push(result)?;
                Ok(())
            }
            _ => Err(ArkError::Generic(format!(
                "Calling non-function value: {:?}",
                func_val
            ))),
        }
    }

    #[inline]
    fn op_make_list(&mut self, size: usize) -> Result<(), ArkError> {
        let mut items = Vec::new();
        for _ in 0..size {
            items.push(
                self.stack
                    .pop()
                    .ok_or_else(|| ArkError::StackUnderflow("MakeList".to_string()))?,
            );
        }
        items.reverse();
        self.push(Value::List(items))?;
        Ok(())
    }

    #[inline]
    fn op_make_struct(&mut self, size: usize) -> Result<(), ArkError> {
        let mut fields = std::collections::HashMap::new();
        for _ in 0..size {
            let key_val = self
                .stack
                .pop()
                .ok_or_else(|| ArkError::StackUnderflow("MakeStruct".to_string()))?;
            let val = self
                .stack
                .pop()
                .ok_or_else(|| ArkError::StackUnderflow("MakeStruct".to_string()))?;

            if let Value::String(key) = key_val {
                fields.insert(key, val);
            } else {
                return Err(ArkError::Generic(format!(
                    "Struct key must be string, got {:?}",
                    key_val
                )));
            }
        }
        self.push(Value::Struct(fields))?;
        Ok(())
    }

    #[inline]
    fn op_destructure(&mut self) -> Result<(), ArkError> {
        let val = self
            .stack
            .pop()
            .ok_or_else(|| ArkError::StackUnderflow("Destructure".to_string()))?;
        if let Value::List(items) = val {
            // Push in reverse order so first item is on top for first Store
            for item in items.into_iter().rev() {
                self.push(item)?;
            }
            Ok(())
        } else {
            Err(ArkError::Generic(format!(
                "Destructure expected List, got {:?}",
                val
            )))
        }
    }

    fn find_var(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Chunk;

    #[test]
    fn test_vm_security_level_zero_allows_anything() {
        let chunk = Chunk::new();
        // Security level 0 should allow "UNTRUSTED" hash
        let vm = VM::new(chunk, "UNTRUSTED", 0);
        assert!(vm.is_ok());
    }

    #[test]
    fn test_vm_security_level_one_blocks_untrusted() {
        let chunk = Chunk::new();
        // Security level 1 should block "UNTRUSTED" hash
        let vm = VM::new(chunk, "UNTRUSTED", 1);
        assert!(matches!(vm, Err(RuntimeError::UntrustedCode)));
    }

    #[test]
    fn test_vm_security_level_one_allows_trusted() {
        let chunk = Chunk::new();
        // Security level 1 requires the hash to be in the blockchain.
        // We register a dummy code and get its hash.
        let hash = crate::blockchain::submit_code("TRUSTED_CODE_PAYLOAD");
        let vm = VM::new(chunk, &hash, 1);
        assert!(vm.is_ok());
    }

    #[test]
    fn test_graph_arena_frame_allocation() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Ret); // Just return unit
        let vm_res = VM::new(chunk, "HASH", 0);
        assert!(vm_res.is_ok());
        let mut vm = vm_res.unwrap();

        // Manually simulate a call frame allocation
        let frame = CallFrame {
            ip: 0,
            chunk: Arc::new(Chunk::new()),
        };
        let idx = vm.heap.alloc(GraphData::Frame(frame));

        // Check ref count
        let node = vm.heap.get(idx).unwrap();
        assert_eq!(node.ref_count, 1);

        // Decref
        vm.heap.decref(idx);

        // Should be gone (or in free list/None)
        let node = vm.heap.get(idx);
        assert!(node.is_none());
    }

    #[test]
    fn test_vm_complex_ops() {
        let mut chunk = Chunk::new();

        // 1. Test MakeList and Destructure
        // Stack: []
        chunk.write(OpCode::Push(Value::Integer(10)));
        chunk.write(OpCode::Push(Value::Integer(20)));
        // Stack: [10, 20]
        chunk.write(OpCode::MakeList(2));
        // Stack: [[10, 20]]
        chunk.write(OpCode::Destructure);
        // Stack: [20, 10] (Top is 10)

        chunk.write(OpCode::Add);
        // Stack: [30] (20 + 10)

        // 2. Test MakeStruct and GetField
        // MakeStruct expects [Value, Key] on stack (Top is Key)
        chunk.write(OpCode::Push(Value::Integer(100)));
        chunk.write(OpCode::Push(Value::String("x".to_string())));
        // Stack: [30, 100, "x"]
        chunk.write(OpCode::MakeStruct(1));
        // Stack: [30, {x: 100}]
        chunk.write(OpCode::GetField("x".to_string()));
        // Stack: [30, 100]
        chunk.write(OpCode::Add);
        // Stack: [130]

        chunk.write(OpCode::Ret);

        let mut vm = VM::new(chunk, "HASH", 0).unwrap();
        let result = vm.run();
        assert_eq!(result.unwrap(), Value::Integer(130));
    }

    #[test]
    fn test_vm_function_call() {
        // Define a function that adds 1 to its argument
        let mut func_chunk = Chunk::new();
        // Stack on entry: [arg]
        // Store arg to 'n'
        func_chunk.write(OpCode::Store("n".to_string()));
        func_chunk.write(OpCode::Load("n".to_string()));
        func_chunk.write(OpCode::Push(Value::Integer(1)));
        func_chunk.write(OpCode::Add);
        func_chunk.write(OpCode::Ret);

        let mut chunk = Chunk::new();
        chunk.write(OpCode::Push(Value::Function(Arc::new(func_chunk))));
        chunk.write(OpCode::Store("add_one".to_string()));

        chunk.write(OpCode::Push(Value::Integer(41)));
        chunk.write(OpCode::Load("add_one".to_string()));
        chunk.write(OpCode::Call(1));
        // Result should be 42

        let mut vm = VM::new(chunk, "HASH", 0).unwrap();
        let result = vm.run();
        assert_eq!(result.unwrap(), Value::Integer(42));
    }

    #[test]
    fn test_vm_call_function_external() {
        // Define a function that adds 1 to its argument
        let mut func_chunk = Chunk::new();
        // Stack on entry: [arg]
        func_chunk.write(OpCode::Store("n".to_string()));
        func_chunk.write(OpCode::Load("n".to_string()));
        func_chunk.write(OpCode::Push(Value::Integer(1)));
        func_chunk.write(OpCode::Add);
        func_chunk.write(OpCode::Ret);

        let mut chunk = Chunk::new();
        chunk.write(OpCode::Push(Value::Function(Arc::new(func_chunk))));
        chunk.write(OpCode::Store("add_one".to_string()));
        chunk.write(OpCode::Ret); // Main finishes

        let mut vm = VM::new(chunk, "HASH", 0).unwrap();
        // Run main to define the function
        let _ = vm.run().unwrap();

        // Now call the function externally
        let result = vm
            .call_public_function("add_one", vec![Value::Integer(99)])
            .unwrap();
        assert_eq!(result, Value::Integer(100));
    }

    #[cfg(test)]
    mod hardening_tests {
        use super::*;
        use crate::bytecode::{Chunk, OpCode};
        use crate::runtime::Value;

        #[test]
        fn test_stack_overflow_protection() {
            let mut chunk = Chunk::new();
            // Push MAX_STACK_DEPTH + 1 values.
            for _ in 0..MAX_STACK_DEPTH + 1 {
                chunk.write(OpCode::Push(Value::Integer(1)));
            }
            chunk.write(OpCode::Ret);

            let mut vm = VM::new(chunk, "HASH", 0).unwrap();
            let result = vm.run();

            assert!(matches!(result, Err(ArkError::StackOverflow)));
        }

        #[test]
        fn test_stack_underflow_on_add() {
            let mut chunk = Chunk::new();
            chunk.write(OpCode::Add); // Stack empty
            chunk.write(OpCode::Ret);

            let mut vm = VM::new(chunk, "HASH", 0).unwrap();
            let result = vm.run();

            match result {
                Err(ArkError::StackUnderflow(op)) => assert_eq!(op, "Add"),
                _ => panic!("Expected StackUnderflow, got {:?}", result),
            }
        }

        #[test]
        fn test_division_by_zero() {
            let mut chunk = Chunk::new();
            chunk.write(OpCode::Push(Value::Integer(10)));
            chunk.write(OpCode::Push(Value::Integer(0)));
            chunk.write(OpCode::Div);
            chunk.write(OpCode::Ret);

            let mut vm = VM::new(chunk, "HASH", 0).unwrap();
            let result = vm.run();

            assert!(matches!(result, Err(ArkError::DivisionByZero)));
        }

        #[test]
        fn test_execution_timeout() {
            let mut chunk = Chunk::new();
            // Infinite loop: Jmp(0)
            chunk.write(OpCode::Jmp(0));

            let mut vm = VM::new(chunk, "HASH", 0).unwrap();
            let result = vm.run();

            assert!(matches!(result, Err(ArkError::ExecutionTimeout)));
        }
    }
}
