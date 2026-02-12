use crate::bytecode::{Chunk, OpCode};
use crate::intrinsics;
use crate::runtime::{RuntimeError, Scope, Value};
use std::rc::Rc;

pub struct CallFrame {
    pub ip: usize,
    pub chunk: Rc<Chunk>,
}

pub struct VM<'a> {
    pub stack: Vec<Value>,
    pub scopes: Vec<Scope<'a>>, // Stack of scopes (Frames)
    pub frames: Vec<CallFrame>,
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

                    if let Some(frame) = self.frames.pop() {
                        self.chunk = frame.chunk;
                        self.ip = frame.ip;
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
                            // Push Frame
                            self.frames.push(CallFrame {
                                ip: self.ip,
                                chunk: self.chunk.clone(),
                            });
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

                    if let Some(frame) = self.frames.pop() {
                        // Restore Context
                        self.chunk = frame.chunk;
                        self.ip = frame.ip;
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
}
