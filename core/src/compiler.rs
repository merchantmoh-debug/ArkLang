use crate::ast::{ArkNode, Expression, Statement};
use crate::bytecode::{Chunk, OpCode};
use crate::runtime::Value;
use std::rc::Rc;

pub struct Compiler {
    pub chunk: Chunk, // Made public for recursive access
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
        }
    }

    pub fn compile(mut self, node: &ArkNode) -> Chunk {
        self.visit(node);
        self.chunk
    }

    fn visit(&mut self, node: &ArkNode) {
        match node {
            ArkNode::Statement(s) => self.visit_stmt(s),
            ArkNode::Expression(e) => {
                self.visit_expr(e);
            }
            _ => {
                // println!("Compiler Warning: Unhandled Top Level Node {:?}", node);
            }
        }
    }

    fn visit_stmt(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Expression(e) => {
                self.visit_expr(e);
                self.chunk.write(OpCode::Pop);
            }
            Statement::Block(stmts) => {
                for s in stmts {
                    self.visit_stmt(s);
                }
            }
            Statement::Let { name, ty: _, value } => {
                self.visit_expr(value);
                self.chunk.write(OpCode::Store(name.clone()));
            }

            Statement::Function(func_def) => {
                // 1. Create new Compiler for function
                let mut func_compiler = Compiler::new();

                // 2. Handle Arguments (Store in reverse order of inputs)
                // inputs: [(name, type), ...]
                // Stack at call time: [arg1, arg2, arg3 (top)]
                // We need to Store arg3, then arg2, then arg1.
                for (arg_name, _) in func_def.inputs.iter().rev() {
                    func_compiler.chunk.write(OpCode::Store(arg_name.clone()));
                }

                // 3. Compile Body
                // body is Box<MastNode>. MastNode has content: ArkNode.
                // We need to visit the content.
                func_compiler.visit(&func_def.body.content);

                // 4. Ensure Return (Optional, if void function falls off)
                func_compiler.chunk.write(OpCode::Push(Value::Unit));
                func_compiler.chunk.write(OpCode::Ret);

                let compiled_chunk = func_compiler.chunk;

                // 5. Emit Push(Value::Function(Rc::new(chunk)))
                let func_val = Value::Function(Rc::new(compiled_chunk));
                self.chunk.write(OpCode::Push(func_val));

                // 6. Store function in variable with its name
                self.chunk.write(OpCode::Store(func_def.name.clone()));
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.visit_expr(condition);
                let jump_idx = self.chunk.code.len();
                self.chunk.write(OpCode::JmpIfFalse(0));

                for s in then_block {
                    self.visit_stmt(s);
                }

                let else_jump_idx = self.chunk.code.len();
                // If there is an else block, we need to jump over it after then block
                if else_block.is_some() {
                    self.chunk.write(OpCode::Jmp(0));
                }

                let after_then_idx = self.chunk.code.len();
                self.chunk.code[jump_idx] = OpCode::JmpIfFalse(after_then_idx);

                if let Some(stmts) = else_block {
                    // The JmpIfFalse above now points to start of else block (after_then_idx)
                    // But wait, if we have else, JmpIfFalse should jump to else start.
                    // My previous logic: code[jump_idx] = JmpIfFalse(after_then_idx).
                    // after_then_idx IS the start of else block code (or Jmp(0) instruction?).
                    // No, else_jump_idx is where Jmp(0) is.
                    // after_then_idx is AFTER Jmp(0). So it is start of else block. Correct.

                    for s in stmts {
                        self.visit_stmt(s);
                    }
                    let end_idx = self.chunk.code.len();
                    self.chunk.code[else_jump_idx] = OpCode::Jmp(end_idx);
                }
            }
            Statement::While { condition, body } => {
                let loop_start_idx = self.chunk.code.len();
                self.visit_expr(condition);
                let jump_idx = self.chunk.code.len();
                self.chunk.write(OpCode::JmpIfFalse(0));
                for s in body {
                    self.visit_stmt(s);
                }
                self.chunk.write(OpCode::Jmp(loop_start_idx));
                let end_idx = self.chunk.code.len();
                self.chunk.code[jump_idx] = OpCode::JmpIfFalse(end_idx);
            }
            Statement::Return(expr) => {
                self.visit_expr(expr);
                self.chunk.write(OpCode::Ret);
            }
            Statement::LetDestructure { names, value } => {
                self.visit_expr(value);
                self.chunk.write(OpCode::Destructure);
                for name in names {
                    self.chunk.write(OpCode::Store(name.clone()));
                }
            }
            Statement::SetField {
                obj_name,
                field,
                value,
            } => {
                self.visit_expr(value);
                self.chunk.write(OpCode::Load(obj_name.clone()));
                self.chunk.write(OpCode::SetField(field.clone()));
                self.chunk.write(OpCode::Store(obj_name.clone()));
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::List(items) => {
                for item in items {
                    self.visit_expr(item);
                }
                self.chunk.write(OpCode::MakeList(items.len()));
            }
            Expression::StructInit { fields } => {
                for (name, expr) in fields {
                    self.visit_expr(expr);
                    self.chunk.write(OpCode::Push(Value::String(name.clone())));
                }
                self.chunk.write(OpCode::MakeStruct(fields.len()));
            }
            Expression::GetField { obj, field } => {
                self.visit_expr(obj);
                self.chunk.write(OpCode::GetField(field.clone()));
            }

            Expression::Literal(s) => {
                self.chunk.write(OpCode::Push(Value::String(s.clone())));
            }

            Expression::Variable(name) => {
                self.chunk.write(OpCode::Load(name.clone()));
            }
            Expression::Call {
                function_hash,
                args,
                ..
            } => {
                // Map intrinsics to OpCodes
                match function_hash.as_str() {
                    "intrinsic_add" | "add" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Add);
                        }
                    }
                    "intrinsic_sub" | "sub" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Sub);
                        }
                    }
                    "intrinsic_mul" | "mul" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Mul);
                        }
                    }
                    "intrinsic_div" | "div" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Div);
                        }
                    }
                    "intrinsic_eq" | "eq" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Eq); // VM handles Eq
                        }
                    }
                    "intrinsic_gt" | "gt" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Gt);
                        }
                    }
                    "intrinsic_lt" | "lt" => {
                        if args.len() == 2 {
                            self.visit_expr(&args[0]);
                            self.visit_expr(&args[1]);
                            self.chunk.write(OpCode::Lt);
                        }
                    }
                    "print" | "intrinsic_print" => {
                        for arg in args {
                            self.visit_expr(arg);
                            self.chunk.write(OpCode::Print);
                        }
                        self.chunk.write(OpCode::Push(Value::Unit));
                    }
                    _ => {
                        // Standard function call (not implemented in VM yet fully)
                        // println!("Compiler Warning: Unknown function call {}", function_hash);
                        for arg in args {
                            self.visit_expr(arg);
                        }
                        // Load function onto stack so VM can pop it
                        self.chunk.write(OpCode::Load(function_hash.clone()));
                        self.chunk.write(OpCode::Call(args.len()));
                    }
                }
            }
        }
    }
}
