use crate::runtime::Value;
// Removed unused imports

/// Source location for debugger mapping
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceLocation {
    pub line: u32,
    pub col: u32,
}

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

    // Enums
    MakeEnum(String, String, usize), // (enum_name, variant, field_count)
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub source_map: Vec<SourceLocation>,
    current_loc: SourceLocation,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            source_map: Vec::new(),
            current_loc: SourceLocation::default(),
        }
    }

    /// Set the current source position for subsequent writes
    pub fn set_source_pos(&mut self, line: u32, col: u32) {
        self.current_loc = SourceLocation { line, col };
    }

    pub fn write(&mut self, op: OpCode) {
        self.code.push(op);
        self.source_map.push(self.current_loc.clone());
    }

    /// Get the source location for a given instruction pointer
    pub fn get_source_loc(&self, ip: usize) -> Option<&SourceLocation> {
        self.source_map.get(ip)
    }
}
