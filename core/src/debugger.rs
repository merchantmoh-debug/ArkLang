/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * Ark Debugger — Breakpoints, stepping, and variable inspection.
 */

use crate::bytecode::Chunk;
use crate::runtime::{Scope, Value};
use std::collections::HashSet;

/// Debugger step mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StepMode {
    /// Run until a breakpoint is hit
    Continue,
    /// Execute one source line (skip function calls)
    StepOver,
    /// Execute one source line (enter function calls)
    StepInto,
    /// Run until current function returns
    StepOut,
}

/// Action returned by the debug hook to the VM
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DebugAction {
    Continue,
    Quit,
}

/// Debugger state machine
pub struct DebugState {
    /// Set of breakpoint line numbers
    pub breakpoints: HashSet<u32>,
    /// Current stepping mode
    pub step_mode: StepMode,
    /// Last source line the debugger stopped at
    pub last_line: u32,
    /// Call depth when step-over was initiated
    pub step_over_depth: usize,
    /// Whether we are in a stepping state (should stop at next line)
    pub stepping: bool,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            breakpoints: HashSet::new(),
            step_mode: StepMode::StepInto,
            last_line: u32::MAX,
            step_over_depth: 0,
            stepping: true,
        }
    }
}

impl DebugState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a breakpoint at the given line number
    pub fn add_breakpoint(&mut self, line: u32) {
        self.breakpoints.insert(line);
    }

    /// Remove a breakpoint at the given line number
    pub fn remove_breakpoint(&mut self, line: u32) {
        self.breakpoints.remove(&line);
    }

    /// Toggle a breakpoint at the given line
    pub fn toggle_breakpoint(&mut self, line: u32) -> bool {
        if self.breakpoints.contains(&line) {
            self.breakpoints.remove(&line);
            false
        } else {
            self.breakpoints.insert(line);
            true
        }
    }

    /// Check whether the debugger should pause at this IP
    pub fn should_break(&mut self, ip: usize, chunk: &Chunk, frame_depth: usize) -> bool {
        let loc = match chunk.get_source_loc(ip) {
            Some(loc) if loc.line > 0 => loc,
            _ => return false,
        };

        let current_line = loc.line;

        // Always skip if we haven't moved to a new line
        if current_line == self.last_line && !self.breakpoints.contains(&current_line) {
            return false;
        }

        // Check breakpoints first (always stop)
        if self.breakpoints.contains(&current_line) {
            return true;
        }

        // Check step mode
        match self.step_mode {
            StepMode::Continue => false,
            StepMode::StepInto => self.stepping && current_line != self.last_line,
            StepMode::StepOver => {
                self.stepping
                    && frame_depth <= self.step_over_depth
                    && current_line != self.last_line
            }
            StepMode::StepOut => self.stepping && frame_depth < self.step_over_depth,
        }
    }
}

/// Format a Value for debugger display (compact, human-readable)
pub fn format_value(val: &Value) -> String {
    match val {
        Value::Integer(i) => format!("{}", i),
        Value::String(s) => format!("\"{}\"", s),
        Value::Boolean(b) => format!("{}", b),
        Value::Unit => "()".to_string(),
        Value::Function(_) => "<function>".to_string(),
        Value::NativeFunction(_) => "<native fn>".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(format_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Buffer(buf) => format!("<buffer {} bytes>", buf.len()),
        Value::Struct(fields) => {
            let inner: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{{ {} }}", inner.join(", "))
        }
        Value::PVec(pv) => format!("{}", pv),
        Value::PMap(pm) => format!("{}", pm),
        Value::LinearObject { id, .. } => format!("<linear {}>", id),
        Value::Return(val) => format!("return({})", format_value(val)),
        Value::EnumValue {
            enum_name,
            variant,
            fields,
        } => {
            if fields.is_empty() {
                format!("{}::{}", enum_name, variant)
            } else {
                let field_strs: Vec<String> = fields.iter().map(format_value).collect();
                format!("{}::{}({})", enum_name, variant, field_strs.join(", "))
            }
        }
    }
}

/// Inspect all visible variables from scope stack (excludes intrinsics)
pub fn inspect_scopes(scopes: &[Scope]) -> Vec<(String, String)> {
    let mut vars = Vec::new();
    for (_depth, scope) in scopes.iter().enumerate().rev() {
        // Scope.variables is private, but we can use scope.get() to check known names
        // Since we can't enumerate private HashMap, we'll capture scope debug output
        let scope_str = format!("{:?}", scope);
        // Parse variable names from debug output
        // This is a pragmatic approach since Scope.variables is private
        if let Some(start) = scope_str.find("variables: {") {
            let rest = &scope_str[start + 12..];
            if let Some(end) = rest.find('}') {
                let var_section = &rest[..end];
                // Extract "name": Value pairs
                for part in var_section.split(',') {
                    let part = part.trim();
                    if let Some(colon_pos) = part.find(':') {
                        let name = part[..colon_pos].trim().trim_matches('"').to_string();
                        // Skip intrinsic names (they start with sys., math., net., etc.)
                        if !name.starts_with("sys.")
                            && !name.starts_with("math.")
                            && !name.starts_with("net.")
                            && !name.starts_with("io.")
                            && !name.starts_with("time.")
                            && !name.starts_with("ai.")
                            && !name.starts_with("governance.")
                            && !name.starts_with("intrinsic_")
                            && name != "print"
                            && name != "exit"
                            && name != "quit"
                        {
                            if let Some(val) = scope.get(&name) {
                                vars.push((name, format_value(&val)));
                            }
                        }
                    }
                }
            }
        }
    }
    vars
}

/// Format a backtrace from the VM's frame/scope state
pub fn format_backtrace(ip: usize, chunk: &Chunk, frame_depth: usize) -> String {
    let loc = chunk
        .get_source_loc(ip)
        .map(|l| format!("line {}, col {}", l.line, l.col))
        .unwrap_or_else(|| format!("ip={}", ip));
    format!("  #{} at {} (ip={})", frame_depth, loc, ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_value_integer() {
        assert_eq!(format_value(&Value::Integer(42)), "42");
    }

    #[test]
    fn test_format_value_string() {
        assert_eq!(
            format_value(&Value::String("hello".to_string())),
            "\"hello\""
        );
    }

    #[test]
    fn test_format_value_list() {
        let list = Value::List(vec![Value::Integer(1), Value::Integer(2)]);
        assert_eq!(format_value(&list), "[1, 2]");
    }

    #[test]
    fn test_format_value_unit() {
        assert_eq!(format_value(&Value::Unit), "()");
    }

    #[test]
    fn test_breakpoint_management() {
        let mut state = DebugState::new();
        assert!(state.breakpoints.is_empty());

        state.add_breakpoint(10);
        assert!(state.breakpoints.contains(&10));

        state.remove_breakpoint(10);
        assert!(!state.breakpoints.contains(&10));

        let added = state.toggle_breakpoint(5);
        assert!(added);
        let removed = state.toggle_breakpoint(5);
        assert!(!removed);
    }

    #[test]
    fn test_should_break_on_breakpoint() {
        let mut state = DebugState::new();
        state.step_mode = StepMode::Continue;
        state.stepping = false;
        state.add_breakpoint(5);

        let mut chunk = Chunk::new();
        chunk.set_source_pos(5, 0);
        chunk.write(crate::bytecode::OpCode::Push(Value::Integer(1)));

        assert!(state.should_break(0, &chunk, 0));
    }

    #[test]
    fn test_should_not_break_without_breakpoint() {
        let mut state = DebugState::new();
        state.step_mode = StepMode::Continue;
        state.stepping = false;
        state.last_line = 3;

        let mut chunk = Chunk::new();
        chunk.set_source_pos(3, 0);
        chunk.write(crate::bytecode::OpCode::Push(Value::Integer(1)));

        assert!(!state.should_break(0, &chunk, 0));
    }
}
