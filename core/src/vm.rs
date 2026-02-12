use crate::bytecode::{Chunk, OpCode};
use crate::intrinsics;
use crate::runtime::{RuntimeError, Scope, Value};
use std::rc::Rc;

// --- GraphArena Implementation ---

#[derive(Debug)]
pub struct GraphArena {
    pub nodes: Vec<Option<GraphNode>>,
    pub free_list: Vec<usize>,
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
            self.nodes[idx] = Some(GraphNode {
                data,
                ref_count: 1,
            });
            idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(GraphNode {
                data,
                ref_count: 1,
            }));
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
    pub chunk: Rc<Chunk>,
}

pub struct VM<'a> {
    pub heap: GraphArena,
    pub stack: Vec<Value>,
    pub scopes: Vec<Scope<'a>>, // Stack of scopes (Frames)
    pub frames: Vec<usize>,     // Stack of frame indices into heap
    pub ip: usize,
    pub chunk: Rc<Chunk>,
    pub security_level: u8,
}

impl<'a> VM<'a> {
    pub fn new(chunk: Chunk, hash: &str, security_level: u8) -> Result<Self, RuntimeError> {
        // Security Check: Verify Code Hash on Blockchain
        if security_level > 0 {
            if !crate::blockchain::verify_code_hash(hash) {
                return Err(RuntimeError::UntrustedCode);
            }
        }

        let mut global_scope = Scope::new();
        crate::intrinsics::IntrinsicRegistry::register_all(&mut global_scope);
        Ok(Self {
            heap: GraphArena::new(),
            stack: Vec::new(),
            scopes: vec![global_scope],
            frames: Vec::new(),
            ip: 0,
            chunk: Rc::new(chunk),
            security_level,
        })
    }

    pub fn run(&mut self) -> Result<Value, String> {
        loop {
            if self.ip >= self.chunk.code.len() {
                // End of chunk.
                if !self.frames.is_empty() {
                    // Implicit Return Unit
                    // Logic matches OpCode::Ret
                    let result = self.stack.pop().unwrap_or(Value::Unit);

                    if let Some(frame_idx) = self.frames.pop() {
                        // Restore Context from Heap
                        if let Some(node) = self.heap.get(frame_idx) {
                            let GraphData::Frame(ref frame) = node.data;
                            self.chunk = frame.chunk.clone();
                            self.ip = frame.ip;
                        } else {
                            return Err(format!("Frame not found at index {}", frame_idx));
                        }

                        // Decrement ref count (and free if 0)
                        self.heap.decref(frame_idx);

                        self.scopes.pop();
                        self.stack.push(result);
                        continue; // Continue loop in previous frame
                    }
                }
                return Ok(Value::Unit);
            }

            let op = &self.chunk.code[self.ip];
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
                OpCode::Push(v) => self.stack.push(v.clone()),
                OpCode::Pop => {
                    self.stack.pop();
                }

                OpCode::Add => self.binary_op(|a, b| intrinsics::intrinsic_add(vec![a, b]))?,
                OpCode::Sub => self.binary_op(|a, b| intrinsics::intrinsic_sub(vec![a, b]))?,
                OpCode::Mul => self.binary_op(|a, b| intrinsics::intrinsic_mul(vec![a, b]))?,
                OpCode::Div => self.binary_op(|a, b| intrinsics::intrinsic_div(vec![a, b]))?,
                OpCode::Mod => self.binary_op(|a, b| intrinsics::intrinsic_mod(vec![a, b]))?,

                OpCode::Eq => self.binary_op(|a, b| intrinsics::intrinsic_eq(vec![a, b]))?,
                OpCode::Neq => self.binary_op_manual(|a, b| {
                    let res =
                        intrinsics::intrinsic_eq(vec![a, b]).map_err(|e| format!("{:?}", e))?;
                    if let Value::Boolean(b) = res {
                        Ok(Value::Boolean(!b))
                    } else {
                        Err("Eq did not return boolean".to_string())
                    }
                })?,
                OpCode::Gt => self.binary_op(|a, b| intrinsics::intrinsic_gt(vec![a, b]))?,
                OpCode::Lt => self.binary_op(|a, b| intrinsics::intrinsic_lt(vec![a, b]))?,
                OpCode::Ge => self.binary_op(|a, b| intrinsics::intrinsic_ge(vec![a, b]))?,
                OpCode::Le => self.binary_op(|a, b| intrinsics::intrinsic_le(vec![a, b]))?,

                OpCode::And => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let res =
                        intrinsics::intrinsic_and(vec![a, b]).map_err(|e| format!("{:?}", e))?;
                    self.stack.push(res);
                }
                OpCode::Or => {
                    let b = self.stack.pop().ok_or("Stack underflow")?;
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let res =
                        intrinsics::intrinsic_or(vec![a, b]).map_err(|e| format!("{:?}", e))?;
                    self.stack.push(res);
                }
                OpCode::Not => {
                    let a = self.stack.pop().ok_or("Stack underflow")?;
                    let res = intrinsics::intrinsic_not(vec![a]).map_err(|e| format!("{:?}", e))?;
                    self.stack.push(res);
                }

                OpCode::Print => {
                    let val = self.stack.pop().ok_or("Stack underflow")?;
                    #[cfg(not(test))]
                    println!("{:?}", val);
                    #[cfg(test)]
                    let _ = val;
                }
                OpCode::Destructure => {
                    let val = self.stack.pop().ok_or("Stack underflow")?;
                    if let Value::List(items) = val {
                        // Push in reverse order so first item is on top for first Store
                        for item in items.into_iter().rev() {
                            self.stack.push(item);
                        }
                    } else {
                        return Err(format!("Destructure expected List, got {:?}", val));
                    }
                }

                OpCode::MakeList(size) => {
                    let mut items = Vec::new();
                    for _ in 0..*size {
                        items.push(self.stack.pop().ok_or("Stack underflow")?);
                    }
                    items.reverse();
                    self.stack.push(Value::List(items));
                }

                OpCode::MakeStruct(size) => {
                    let mut fields = std::collections::HashMap::new();
                    for _ in 0..*size {
                        let key_val = self.stack.pop().ok_or("Stack underflow")?;
                        let val = self.stack.pop().ok_or("Stack underflow")?;

                        if let Value::String(key) = key_val {
                            fields.insert(key, val);
                        } else {
                            return Err(format!("Struct key must be string, got {:?}", key_val));
                        }
                    }
                    self.stack.push(Value::Struct(fields));
                }

                OpCode::GetField(field) => {
                    let obj = self.stack.pop().ok_or("Stack underflow")?;
                    if let Value::Struct(mut fields) = obj {
                        if let Some(val) = fields.remove(field) {
                            self.stack.push(val);
                        } else {
                            return Err(format!("Field '{}' not found in struct", field));
                        }
                    } else {
                        return Err(format!("GetField expected Struct, got {:?}", obj));
                    }
                }

                OpCode::SetField(field) => {
                    let obj = self.stack.pop().ok_or("Stack underflow")?;
                    let val = self.stack.pop().ok_or("Stack underflow")?;

                    if let Value::Struct(mut fields) = obj {
                        fields.insert(field.clone(), val);
                        self.stack.push(Value::Struct(fields));
                    } else {
                        return Err(format!("SetField expected Struct, got {:?}", obj));
                    }
                }

                OpCode::Load(name) => {
                    let val = self
                        .find_var(name)
                        .ok_or_else(|| format!("Variable not found: {}", name))?;
                    self.stack.push(val);
                }
                OpCode::Store(name) => {
                    let val = self.stack.last().ok_or("Stack underflow")?.clone();
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
                    let val = self.stack.pop().ok_or("Stack underflow")?;
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

                OpCode::Call(arg_count) => {
                    let func_val = self.stack.pop().ok_or("Stack underflow during call")?;
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
                        }
                        Value::NativeFunction(func) => {
                            let mut args = Vec::new();
                            for _ in 0..*arg_count {
                                args.push(
                                    self.stack
                                        .pop()
                                        .ok_or("Stack underflow during native call")?,
                                );
                            }
                            // Arguments popped in reverse order (last arg first)
                            // We need them in forward order [arg1, arg2]
                            args.reverse();

                            let result = func(args).map_err(|e| format!("{:?}", e))?;
                            self.stack.push(result);
                        }
                        _ => {
                            return Err(format!("Calling non-function value: {:?}", func_val));
                        }
                    }
                }

                OpCode::Ret => {
                    let result = self.stack.pop().unwrap_or(Value::Unit);

                    if let Some(frame_idx) = self.frames.pop() {
                        // Restore Context from Heap
                        if let Some(node) = self.heap.get(frame_idx) {
                            let GraphData::Frame(ref frame) = node.data;
                            self.chunk = frame.chunk.clone();
                            self.ip = frame.ip;
                        } else {
                            return Err(format!("Frame not found at index {}", frame_idx));
                        }

                        // Decrement ref count (and free if 0)
                        self.heap.decref(frame_idx);

                        // Pop Scope
                        self.scopes.pop();
                        // Push Result
                        self.stack.push(result);
                    } else {
                        // Return from main
                        return Ok(result);
                    }
                }
            }
        }
    }

    fn binary_op<F>(&mut self, op_fn: F) -> Result<(), String>
    where
        F: Fn(Value, Value) -> Result<Value, RuntimeError>,
    {
        let b = self.stack.pop().ok_or("Stack underflow")?;
        let a = self.stack.pop().ok_or("Stack underflow")?;
        let res = op_fn(a, b).map_err(|e| format!("{:?}", e))?;
        self.stack.push(res);
        Ok(())
    }

    fn binary_op_manual<F>(&mut self, op_fn: F) -> Result<(), String>
    where
        F: Fn(Value, Value) -> Result<Value, String>,
    {
        let b = self.stack.pop().ok_or("Stack underflow")?;
        let a = self.stack.pop().ok_or("Stack underflow")?;
        let res = op_fn(a, b)?;
        self.stack.push(res);
        Ok(())
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
        // Security level 1 should allow trusted hash (anything not "UNTRUSTED" in our mock)
        let vm = VM::new(chunk, "TRUSTED_HASH", 1);
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
            chunk: Rc::new(Chunk::new()),
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
}
