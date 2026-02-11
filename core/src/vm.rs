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
}

impl<'a> VM<'a> {
    pub fn new(chunk: Chunk) -> Self {
        let mut global_scope = Scope::new();
        crate::intrinsics::IntrinsicRegistry::register_all(&mut global_scope);
        Self {
            stack: Vec::new(),
            scopes: vec![global_scope],
            frames: Vec::new(),
            ip: 0,
            chunk: Rc::new(chunk),
        }
    }

    pub fn run(&mut self) -> Result<Value, String> {
        loop {
            if self.ip >= self.chunk.code.len() {
                // If main chunk finishes, we return.
                // But what if we just returned from a function and frames is empty?
                // OpCode::Ret handles return.
                // If we fall off end of main, we return Unit.
                return Ok(Value::Unit);
            }

            let op = &self.chunk.code[self.ip];
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

                _ => return Err(format!("Unimplemented opcode: {:?}", op)),
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
