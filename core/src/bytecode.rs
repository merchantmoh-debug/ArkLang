use crate::runtime::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    // Stack Manipulation
    Push(Value),
    Pop,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Neq,
    Gt,
    Lt,
    Ge,
    Le,

    // Logic
    And,
    Or,
    Not,

    // Variables
    Load(String),
    Store(String),

    // Control Flow
    Jmp(usize),
    JmpIfFalse(usize),

    // Functions
    Call(usize), // Argument count
    Ret,

    // System
    Print,
    Destructure,

    // Types
    MakeList(usize),
    MakeStruct(usize),
    GetField(String),
    SetField(String),
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>, // Using OpCode::Push(Value) directly for now, optimization later
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
        }
    }

    pub fn write(&mut self, op: OpCode) {
        self.code.push(op);
    }
}
