/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 *
 * 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
 * General Public License v3.0. If you link to this code, your ENTIRE
 * application must be open-sourced under AGPLv3.
 *
 * 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
 * from Sovereign Systems.
 *
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 * NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.
 */

use crate::bytecode::Chunk;

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};
use thiserror::Error;

// --- Resource Tracker ---

type ResourceEntry = (String, Box<dyn FnOnce() + Send>);

pub struct ResourceTracker {
    resources: Mutex<HashMap<usize, ResourceEntry>>,
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            resources: Mutex::new(HashMap::new()),
        }
    }

    pub fn register<F>(&self, id: usize, resource_type: &str, cleanup: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut map = self.resources.lock().unwrap();
        map.insert(id, (resource_type.to_string(), Box::new(cleanup)));
    }

    pub fn release(&self, id: usize) {
        let callback = {
            let mut map = self.resources.lock().unwrap();
            map.remove(&id)
        };
        if let Some((_, cleanup)) = callback {
            cleanup();
        }
    }

    pub fn cleanup_all(&self) {
        // Fix: Drain while holding lock, but run callbacks AFTER lock is released.
        // This prevents deadlock if callback tries to access RESOURCE_TRACKER.
        let resources_to_clean: Vec<(usize, ResourceEntry)> = {
            let mut map = self.resources.lock().unwrap();
            map.drain().collect()
        };

        for (id, (name, cleanup)) in resources_to_clean {
            println!(
                "Warning: Resource {} (type {}) was not closed explicitly.",
                id, name
            );
            cleanup();
        }
    }
}

lazy_static! {
    pub static ref RESOURCE_TRACKER: ResourceTracker = ResourceTracker::new();
}

// --- Value Pool ---

pub struct ValuePool;

thread_local! {
    static INT_POOL: Vec<Value> = {
        let mut pool = Vec::with_capacity(256);
        for i in -128..128 {
            pool.push(Value::Integer(i));
        }
        pool
    };
}

impl ValuePool {
    /// Returns a Value::Integer. Uses cached instance if within [-128, 127].
    pub fn pool_int(i: i64) -> Value {
        if (-128..128).contains(&i) {
            INT_POOL.with(|pool| pool[(i + 128) as usize].clone())
        } else {
            Value::Integer(i)
        }
    }

    pub fn pool_bool(b: bool) -> Value {
        Value::Boolean(b)
    }

    pub fn pool_str(s: &str) -> Result<Value, RuntimeError> {
        MemoryManager::track_alloc(s.len())?;
        // Cloning String allocates. True pooling requires changing Value to use Rc/Arc.
        // We provide this API for future optimization.
        Ok(Value::String(s.to_string()))
    }

    pub fn unit() -> Value {
        Value::Unit
    }
}

// --- Memory Management ---

pub static MAX_MEMORY_MB: AtomicUsize = AtomicUsize::new(256);
pub static CURRENT_MEMORY_USAGE: AtomicUsize = AtomicUsize::new(0);

pub struct MemoryManager;

impl MemoryManager {
    pub fn track_alloc(bytes: usize) -> Result<(), RuntimeError> {
        let max_mb = MAX_MEMORY_MB.load(Ordering::Relaxed);
        let max_bytes = max_mb * 1024 * 1024;

        // Speculative allocation to avoid race condition
        let previous = CURRENT_MEMORY_USAGE.fetch_add(bytes, Ordering::Relaxed);
        let new_usage = previous + bytes;

        if new_usage > max_bytes {
            // Rollback
            CURRENT_MEMORY_USAGE.fetch_sub(bytes, Ordering::Relaxed);
            return Err(RuntimeError::AllocationError(
                "Memory limit exceeded".to_string(),
            ));
        }

        RUNTIME_STATS
            .total_allocations
            .fetch_add(1, Ordering::Relaxed);

        // Track peak memory
        let mut peak = RUNTIME_STATS.peak_memory_bytes.load(Ordering::Relaxed);
        while new_usage > peak {
            match RUNTIME_STATS.peak_memory_bytes.compare_exchange_weak(
                peak,
                new_usage,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
        Ok(())
    }

    pub fn track_dealloc(bytes: usize) {
        CURRENT_MEMORY_USAGE.fetch_sub(bytes, Ordering::Relaxed);
    }
}

// --- Runtime Stats ---

pub struct RuntimeStats {
    pub total_instructions: AtomicUsize,
    pub total_allocations: AtomicUsize,
    pub peak_memory_bytes: AtomicUsize,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeStats {
    pub fn new() -> Self {
        Self {
            total_instructions: AtomicUsize::new(0),
            total_allocations: AtomicUsize::new(0),
            peak_memory_bytes: AtomicUsize::new(0),
        }
    }

    pub fn stats(&self) -> String {
        format!(
            "Runtime Stats:\n  Instructions: {}\n  Allocations: {}\n  Peak Memory: {} bytes",
            self.total_instructions.load(Ordering::Relaxed),
            self.total_allocations.load(Ordering::Relaxed),
            self.peak_memory_bytes.load(Ordering::Relaxed)
        )
    }
}

lazy_static! {
    pub static ref RUNTIME_STATS: RuntimeStats = RuntimeStats::new();
}

// --- Graceful Shutdown ---

pub static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub fn init_shutdown_handler() {
    ctrlc::set_handler(move || {
        println!("Received Ctrl+C, initiating graceful shutdown...");
        shutdown();

        // Wait up to 5 seconds for threads (simulated by sleep)
        std::thread::sleep(std::time::Duration::from_secs(5));

        println!("Force exiting.");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");
}

pub fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    RESOURCE_TRACKER.cleanup_all();
    if std::env::var("ARK_RUNTIME_STATS").is_ok() {
        println!("{}", RUNTIME_STATS.stats());
    }
}

// --- Existing Types ---

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    String(String),
    Boolean(bool),
    Unit,
    /// A linear object at runtime. Wraps internal data.
    LinearObject {
        id: String,
        typename: String,
        payload: String, // Simplified representation
    },
    Function(Arc<Chunk>), // Bytecode Function
    NativeFunction(NativeFn),
    List(Vec<Value>),
    Buffer(Vec<u8>),
    Struct(HashMap<String, Value>),
    /// Control Flow: Return value wrapper
    Return(Box<Value>),
}

pub type NativeFn = fn(Vec<Value>) -> Result<Value, RuntimeError>;

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            (Value::LinearObject { id: a, .. }, Value::LinearObject { id: b, .. }) => a == b,
            (Value::Function(a), Value::Function(b)) => Arc::ptr_eq(a, b),
            (Value::NativeFunction(a), Value::NativeFunction(b)) => *a as usize == *b as usize,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Buffer(a), Value::Buffer(b)) => a == b,
            (Value::Struct(a), Value::Struct(b)) => a == b, // Note: HashMap comparisons can be slow
            (Value::Return(a), Value::Return(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    pub fn is_linear(&self) -> bool {
        match self {
            Value::Integer(_)
            | Value::Boolean(_)
            | Value::Unit
            | Value::Function(_)
            | Value::NativeFunction(_)
            | Value::String(_) => false,
            Value::List(_) | Value::LinearObject { .. } | Value::Buffer(_) | Value::Struct(_) => {
                true
            }
            Value::Return(val) => val.is_linear(), // Recursive check
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    variables: HashMap<String, Value>,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Default for Scope<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Scope<'a> {
    pub fn new() -> Self {
        Scope {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: &'a Scope<'a>) -> Self {
        Scope {
            variables: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        match self.variables.get(name) {
            Some(v) => Some(v.clone()),
            None => match &self.parent {
                Some(p) => p.get(name),
                None => None,
            },
        }
    }

    pub fn get_or_move(&mut self, name: &str) -> Option<Value> {
        // 1. Try Local
        if let Some(v) = self.variables.get(name) {
            if v.is_linear() {
                return self.variables.remove(name);
            } else {
                return Some(v.clone());
            }
        }
        // 2. Try Parent (Only for Shared types, or implicit clone of Linear if allowed/unsafe)
        // Note: Moving out of parent is impossible with &Scope.
        // Strict Linear Type Checker prevents capturing Linear by reference if logic is sound.
        if let Some(parent) = &self.parent {
            return parent.get(name);
        }
        None
    }

    pub fn take(&mut self, name: &str) -> Option<Value> {
        if let Some(v) = self.variables.remove(name) {
            return Some(v);
        }
        None
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Variable not found: {0}")]
    VariableNotFound(String),
    #[error("Type mismatch: expected {0}, got {1:?}")]
    TypeMismatch(String, Value),
    #[error("Not executable")]
    NotExecutable,
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("System Lockout: Recursion Limit Exceeded (Vertigo Check)")]
    RecursionLimitExceeded,
    #[error("System Lockout: Untrusted Code Hash")]
    UntrustedCode,
    #[error("Allocation failed: {0}")]
    AllocationError(String),
    #[error("Resource error: {0}")]
    ResourceError(String),
}

impl From<std::io::Error> for RuntimeError {
    fn from(err: std::io::Error) -> Self {
        RuntimeError::ResourceError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_resource_tracker_lifecycle() {
        let tracker = ResourceTracker::new();
        let flag = Arc::new(Mutex::new(false));
        let flag_clone = flag.clone();

        tracker.register(1, "test_resource", move || {
            let mut f = flag_clone.lock().unwrap();
            *f = true;
        });

        // Release manually
        tracker.release(1);
        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_resource_tracker_cleanup_unreleased() {
        let tracker = ResourceTracker::new();
        let flag = Arc::new(Mutex::new(false));
        let flag_clone = flag.clone();

        tracker.register(2, "test_resource_2", move || {
            let mut f = flag_clone.lock().unwrap();
            *f = true;
        });

        // Cleanup all (simulating shutdown)
        tracker.cleanup_all();
        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_value_pool_integer_caching() {
        let v1 = ValuePool::pool_int(42);
        let v2 = ValuePool::pool_int(42);

        assert_eq!(v1, v2);

        if let Value::Integer(i) = v1 {
            assert_eq!(i, 42);
        } else {
            panic!("Expected Integer");
        }

        let v3 = ValuePool::pool_int(1000); // Outside cache
        if let Value::Integer(i) = v3 {
            assert_eq!(i, 1000);
        } else {
            panic!("Expected Integer");
        }
    }

    #[test]
    fn test_memory_limit_enforcement() {
        // Reset usage for test
        CURRENT_MEMORY_USAGE.store(0, Ordering::Relaxed);
        MAX_MEMORY_MB.store(1, Ordering::Relaxed); // 1 MB limit

        // Alloc 500KB - OK
        assert!(MemoryManager::track_alloc(500 * 1024).is_ok());

        // Alloc 600KB - Should fail (Total 1.1MB > 1MB)
        assert!(MemoryManager::track_alloc(600 * 1024).is_err());

        // Dealloc 500KB
        MemoryManager::track_dealloc(500 * 1024);

        // Alloc 600KB - Should pass now (Total 600KB)
        assert!(MemoryManager::track_alloc(600 * 1024).is_ok());
    }

    #[test]
    fn test_value_pool_string_alloc_limit() {
        // Reset usage for test
        CURRENT_MEMORY_USAGE.store(0, Ordering::Relaxed);
        MAX_MEMORY_MB.store(1, Ordering::Relaxed); // 1 MB limit

        // Test normal alloc
        assert!(ValuePool::pool_str("short string").is_ok());

        // Test oversize alloc
        // Create a string that exceeds 1MB
        let large_string = "a".repeat(1024 * 1024 + 10);
        let result = ValuePool::pool_str(&large_string);
        assert!(result.is_err());
        match result {
            Err(RuntimeError::AllocationError(_)) => (), // OK
            _ => panic!("Expected AllocationError"),
        }
    }

    #[test]
    fn test_runtime_stats_counting() {
        // Reset
        RUNTIME_STATS.total_allocations.store(0, Ordering::Relaxed);

        MemoryManager::track_alloc(100).unwrap();
        assert_eq!(RUNTIME_STATS.total_allocations.load(Ordering::Relaxed), 1);

        MemoryManager::track_alloc(100).unwrap();
        assert_eq!(RUNTIME_STATS.total_allocations.load(Ordering::Relaxed), 2);

        let stats = RUNTIME_STATS.stats();
        assert!(stats.contains("Allocations: 2"));
    }
}
