/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark agent graceful shutdown system.
 */

//! Graceful shutdown — ordered subsystem teardown for clean exit.
//!
//! When Ark receives a shutdown signal (SIGTERM, Ctrl+C, API call), this
//! module orchestrates an ordered shutdown sequence to prevent data loss
//! and ensure clean resource cleanup.
//!
//! Shutdown sequence (order matters):
//! 1. Stop accepting new requests (mark as draining)
//! 2. Broadcast shutdown to connected clients
//! 3. Wait for in-flight agent loops to complete (with timeout)
//! 4. Close browser/sandbox sessions
//! 5. Stop MCP connections
//! 6. Stop heartbeat/background tasks
//! 7. Flush audit log
//! 8. Close database connections
//! 9. Exit

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// ShutdownPhase
// ---------------------------------------------------------------------------

/// Shutdown phase identifiers (in execution order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[repr(u8)]
pub enum ShutdownPhase {
    Running = 0,
    Draining = 1,
    BroadcastingShutdown = 2,
    WaitingForAgents = 3,
    ClosingSandboxes = 4,
    ClosingMcp = 5,
    StoppingBackground = 6,
    FlushingAudit = 7,
    ClosingDatabase = 8,
    Complete = 9,
}

impl std::fmt::Display for ShutdownPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Draining => write!(f, "draining"),
            Self::BroadcastingShutdown => write!(f, "broadcasting_shutdown"),
            Self::WaitingForAgents => write!(f, "waiting_for_agents"),
            Self::ClosingSandboxes => write!(f, "closing_sandboxes"),
            Self::ClosingMcp => write!(f, "closing_mcp"),
            Self::StoppingBackground => write!(f, "stopping_background"),
            Self::FlushingAudit => write!(f, "flushing_audit"),
            Self::ClosingDatabase => write!(f, "closing_database"),
            Self::Complete => write!(f, "complete"),
        }
    }
}

impl ShutdownPhase {
    /// Convert u8 representation back to enum.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Running,
            1 => Self::Draining,
            2 => Self::BroadcastingShutdown,
            3 => Self::WaitingForAgents,
            4 => Self::ClosingSandboxes,
            5 => Self::ClosingMcp,
            6 => Self::StoppingBackground,
            7 => Self::FlushingAudit,
            8 => Self::ClosingDatabase,
            _ => Self::Complete,
        }
    }
}

// ---------------------------------------------------------------------------
// ShutdownConfig
// ---------------------------------------------------------------------------

/// Configuration for graceful shutdown.
#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight requests to complete.
    pub drain_timeout: Duration,
    /// Maximum time to wait for agent loops to finish.
    pub agent_timeout: Duration,
    /// Maximum time for the entire shutdown sequence.
    pub total_timeout: Duration,
    /// Whether to broadcast a shutdown message to connected clients.
    pub broadcast_shutdown: bool,
    /// Human-readable reason for shutdown.
    pub shutdown_reason: String,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            drain_timeout: Duration::from_secs(30),
            agent_timeout: Duration::from_secs(60),
            total_timeout: Duration::from_secs(120),
            broadcast_shutdown: true,
            shutdown_reason: "System shutdown".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// PhaseLog
// ---------------------------------------------------------------------------

/// Log entry for a completed shutdown phase.
#[derive(Debug, Clone, Serialize)]
pub struct PhaseLog {
    pub phase: ShutdownPhase,
    pub duration_ms: u64,
    pub success: bool,
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// ShutdownStatus
// ---------------------------------------------------------------------------

/// Shutdown progress snapshot (for API responses / monitoring).
#[derive(Debug, Clone, Serialize)]
pub struct ShutdownStatus {
    pub is_shutting_down: bool,
    pub current_phase: String,
    pub elapsed_secs: f64,
    pub reason: String,
    pub phases_completed: Vec<PhaseLog>,
}

// ---------------------------------------------------------------------------
// ShutdownCoordinator
// ---------------------------------------------------------------------------

/// Tracks the state of a graceful shutdown in progress.
///
/// Thread-safe via atomics and internal mutexes.
pub struct ShutdownCoordinator {
    is_shutting_down: AtomicBool,
    current_phase: AtomicU8,
    started_at: std::sync::Mutex<Option<Instant>>,
    config: ShutdownConfig,
    phase_log: std::sync::Mutex<Vec<PhaseLog>>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator.
    pub fn new(config: ShutdownConfig) -> Self {
        Self {
            is_shutting_down: AtomicBool::new(false),
            current_phase: AtomicU8::new(ShutdownPhase::Running as u8),
            started_at: std::sync::Mutex::new(None),
            config,
            phase_log: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Check if shutdown is in progress.
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Relaxed)
    }

    /// Initiate shutdown. Returns `false` if already shutting down.
    pub fn initiate(&self) -> bool {
        if self.is_shutting_down.swap(true, Ordering::SeqCst) {
            return false;
        }
        *self.started_at.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        true
    }

    /// Get the current shutdown phase.
    pub fn current_phase(&self) -> ShutdownPhase {
        ShutdownPhase::from_u8(self.current_phase.load(Ordering::Relaxed))
    }

    /// Advance to the next phase. Records timing for the completed phase.
    pub fn advance_phase(&self, next: ShutdownPhase, success: bool, message: Option<String>) {
        let current = self.current_phase();
        let elapsed = self
            .started_at
            .lock()
            .expect("unexpected failure")
            .map(|s| s.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let log = PhaseLog {
            phase: current,
            duration_ms: elapsed,
            success,
            message,
        };

        self.phase_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(log);
        self.current_phase.store(next as u8, Ordering::SeqCst);
    }

    /// Get a snapshot of shutdown status.
    pub fn status(&self) -> ShutdownStatus {
        let elapsed = self
            .started_at
            .lock()
            .expect("unexpected failure")
            .map(|s| s.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        ShutdownStatus {
            is_shutting_down: self.is_shutting_down(),
            current_phase: self.current_phase().to_string(),
            elapsed_secs: elapsed,
            reason: self.config.shutdown_reason.clone(),
            phases_completed: self
                .phase_log
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        }
    }

    /// Check if the total timeout has been exceeded.
    pub fn is_timeout_exceeded(&self) -> bool {
        self.started_at
            .lock()
            .expect("unexpected failure")
            .map(|s| s.elapsed() > self.config.total_timeout)
            .unwrap_or(false)
    }

    /// Get the drain timeout duration.
    pub fn drain_timeout(&self) -> Duration {
        self.config.drain_timeout
    }

    /// Get the agent timeout duration.
    pub fn agent_timeout(&self) -> Duration {
        self.config.agent_timeout
    }

    /// Whether to broadcast shutdown to clients.
    pub fn should_broadcast(&self) -> bool {
        self.config.broadcast_shutdown
    }

    /// Get the shutdown reason.
    pub fn shutdown_reason(&self) -> &str {
        &self.config.shutdown_reason
    }

    /// Build a JSON-compatible shutdown message.
    pub fn shutdown_message_json(&self) -> String {
        let status = self.status();
        serde_json::json!({
            "type": "shutdown",
            "reason": status.reason,
            "phase": status.current_phase,
        })
        .to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_config_defaults() {
        let config = ShutdownConfig::default();
        assert_eq!(config.drain_timeout, Duration::from_secs(30));
        assert_eq!(config.agent_timeout, Duration::from_secs(60));
        assert_eq!(config.total_timeout, Duration::from_secs(120));
        assert!(config.broadcast_shutdown);
        assert_eq!(config.shutdown_reason, "System shutdown");
    }

    #[test]
    fn test_coordinator_not_shutting_down_initially() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        assert!(!coord.is_shutting_down());
        assert_eq!(coord.current_phase(), ShutdownPhase::Running);
    }

    #[test]
    fn test_initiate_shutdown() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        assert!(coord.initiate());
        assert!(coord.is_shutting_down());
    }

    #[test]
    fn test_double_initiate_returns_false() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        assert!(coord.initiate());
        assert!(!coord.initiate());
        assert!(coord.is_shutting_down());
    }

    #[test]
    fn test_phase_advancement() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        coord.initiate();
        assert_eq!(coord.current_phase(), ShutdownPhase::Running);

        coord.advance_phase(ShutdownPhase::Draining, true, None);
        assert_eq!(coord.current_phase(), ShutdownPhase::Draining);

        coord.advance_phase(ShutdownPhase::BroadcastingShutdown, true, None);
        assert_eq!(coord.current_phase(), ShutdownPhase::BroadcastingShutdown);

        coord.advance_phase(ShutdownPhase::WaitingForAgents, true, None);
        assert_eq!(coord.current_phase(), ShutdownPhase::WaitingForAgents);

        coord.advance_phase(ShutdownPhase::Complete, true, None);
        assert_eq!(coord.current_phase(), ShutdownPhase::Complete);
    }

    #[test]
    fn test_phase_display_names() {
        assert_eq!(ShutdownPhase::Running.to_string(), "running");
        assert_eq!(ShutdownPhase::Draining.to_string(), "draining");
        assert_eq!(
            ShutdownPhase::BroadcastingShutdown.to_string(),
            "broadcasting_shutdown"
        );
        assert_eq!(
            ShutdownPhase::WaitingForAgents.to_string(),
            "waiting_for_agents"
        );
        assert_eq!(
            ShutdownPhase::ClosingSandboxes.to_string(),
            "closing_sandboxes"
        );
        assert_eq!(ShutdownPhase::ClosingMcp.to_string(), "closing_mcp");
        assert_eq!(
            ShutdownPhase::StoppingBackground.to_string(),
            "stopping_background"
        );
        assert_eq!(ShutdownPhase::FlushingAudit.to_string(), "flushing_audit");
        assert_eq!(
            ShutdownPhase::ClosingDatabase.to_string(),
            "closing_database"
        );
        assert_eq!(ShutdownPhase::Complete.to_string(), "complete");
    }

    #[test]
    fn test_status_snapshot() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        let status = coord.status();
        assert!(!status.is_shutting_down);
        assert_eq!(status.current_phase, "running");
        assert_eq!(status.reason, "System shutdown");
        assert!(status.phases_completed.is_empty());
    }

    #[test]
    fn test_timeout_check() {
        let config = ShutdownConfig {
            total_timeout: Duration::from_millis(1),
            ..Default::default()
        };
        let coord = ShutdownCoordinator::new(config);
        assert!(!coord.is_timeout_exceeded());
        coord.initiate();
        std::thread::sleep(Duration::from_millis(10));
        assert!(coord.is_timeout_exceeded());
    }

    #[test]
    fn test_shutdown_message_json() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        coord.initiate();
        let msg = coord.shutdown_message_json();
        let parsed: serde_json::Value = serde_json::from_str(&msg).expect("valid JSON");
        assert_eq!(parsed["type"], "shutdown");
        assert_eq!(parsed["reason"], "System shutdown");
        assert_eq!(parsed["phase"], "running");
    }

    #[test]
    fn test_shutdown_reason() {
        let config = ShutdownConfig {
            shutdown_reason: "Maintenance window".to_string(),
            ..Default::default()
        };
        let coord = ShutdownCoordinator::new(config);
        assert_eq!(coord.shutdown_reason(), "Maintenance window");
    }

    #[test]
    fn test_phase_log_recording() {
        let coord = ShutdownCoordinator::new(ShutdownConfig::default());
        coord.initiate();

        coord.advance_phase(ShutdownPhase::Draining, true, None);
        coord.advance_phase(
            ShutdownPhase::BroadcastingShutdown,
            false,
            Some("Broadcast failed".to_string()),
        );

        let status = coord.status();
        assert_eq!(status.phases_completed.len(), 2);
        assert_eq!(status.phases_completed[0].phase, ShutdownPhase::Running);
        assert!(status.phases_completed[0].success);
        assert_eq!(status.phases_completed[1].phase, ShutdownPhase::Draining);
        assert!(!status.phases_completed[1].success);
        assert_eq!(
            status.phases_completed[1].message.as_deref(),
            Some("Broadcast failed")
        );
    }

    #[test]
    fn test_all_phases_ordered() {
        let phases = [
            ShutdownPhase::Running,
            ShutdownPhase::Draining,
            ShutdownPhase::BroadcastingShutdown,
            ShutdownPhase::WaitingForAgents,
            ShutdownPhase::ClosingSandboxes,
            ShutdownPhase::ClosingMcp,
            ShutdownPhase::StoppingBackground,
            ShutdownPhase::FlushingAudit,
            ShutdownPhase::ClosingDatabase,
            ShutdownPhase::Complete,
        ];
        for i in 1..phases.len() {
            assert!(
                phases[i] > phases[i - 1],
                "{:?} should be > {:?}",
                phases[i],
                phases[i - 1]
            );
        }
        assert_eq!(phases.len(), 10);
    }

    #[test]
    fn test_phase_from_u8_roundtrip() {
        for val in 0..=9u8 {
            let phase = ShutdownPhase::from_u8(val);
            assert_eq!(phase as u8, val);
        }
        // Out of range → Complete
        assert_eq!(ShutdownPhase::from_u8(255), ShutdownPhase::Complete);
    }
}
