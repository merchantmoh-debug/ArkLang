/*
 * Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
 *
 * This file is part of the Ark Sovereign Compiler.
 *
 * LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
 * PATENT NOTICE: Protected by US Patent App #63/935,467.
 *
 * Original implementation derived from OpenFang (MIT/Apache-2.0).
 * Adapted for the Ark retry/backoff system.
 */

//! Generic retry with exponential backoff and jitter.
//!
//! Provides a configurable retry utility for LLM API calls, network
//! operations, channel message delivery, and any other fallible operation.
//!
//! Uses `std::time::SystemTime` UNIX nanos as a lightweight pseudo-random
//! source for jitter, avoiding the `rand` crate dependency.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts (including the first try).
    pub max_attempts: u32,
    /// Minimum delay between retries in milliseconds.
    pub min_delay_ms: u64,
    /// Maximum delay between retries in milliseconds.
    pub max_delay_ms: u64,
    /// Jitter factor (0.0 = no jitter, 1.0 = full jitter).
    pub jitter: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            min_delay_ms: 300,
            max_delay_ms: 30_000,
            jitter: 0.2,
        }
    }
}

/// Result of a retry operation.
#[derive(Debug)]
pub enum RetryOutcome<T, E> {
    /// The operation succeeded.
    Success {
        result: T,
        /// Total number of attempts made (1 = first try succeeded).
        attempts: u32,
    },
    /// All retries exhausted without success.
    Exhausted { last_error: E, attempts: u32 },
}

// ---------------------------------------------------------------------------
// Backoff computation
// ---------------------------------------------------------------------------

/// Compute the delay for a given attempt (0-indexed).
///
/// Formula: `min(min_delay * 2^attempt, max_delay) * (1 + random * jitter)`
pub fn compute_backoff(config: &RetryConfig, attempt: u32) -> u64 {
    let base = config
        .min_delay_ms
        .saturating_mul(1u64.checked_shl(attempt).unwrap_or(u64::MAX));
    let capped = base.min(config.max_delay_ms);

    if config.jitter <= 0.0 {
        return capped;
    }

    let frac = pseudo_random_fraction();
    let jitter_offset = (capped as f64) * frac * config.jitter;
    let with_jitter = (capped as f64) + jitter_offset;

    (with_jitter as u64).min(config.max_delay_ms)
}

/// Return a pseudo-random fraction in `[0, 1)` using system time nanos.
/// NOT cryptographically secure — good enough for jitter.
fn pseudo_random_fraction() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // Knuth multiplicative hash for bit mixing
    let mixed = nanos.wrapping_mul(2654435761);
    (mixed as f64) / (u32::MAX as f64)
}

// ---------------------------------------------------------------------------
// Synchronous retry
// ---------------------------------------------------------------------------

/// Execute a synchronous operation with retry.
///
/// Uses `std::thread::sleep` between attempts. For async retry, wrap this
/// or use Ark's native WASM async runtime.
pub fn retry_sync<F, T, E, P, H>(
    config: &RetryConfig,
    mut operation: F,
    should_retry: P,
    retry_after_hint: H,
) -> RetryOutcome<T, E>
where
    F: FnMut() -> Result<T, E>,
    P: Fn(&E) -> bool,
    H: Fn(&E) -> Option<u64>,
    E: std::fmt::Debug,
{
    let max = config.max_attempts.max(1);

    for attempt in 0..max {
        match operation() {
            Ok(result) => {
                return RetryOutcome::Success {
                    result,
                    attempts: attempt + 1,
                };
            }
            Err(err) => {
                let is_last = attempt + 1 >= max;

                if is_last || !should_retry(&err) {
                    return RetryOutcome::Exhausted {
                        last_error: err,
                        attempts: attempt + 1,
                    };
                }

                let delay_ms = if let Some(hinted) = retry_after_hint(&err) {
                    hinted.min(config.max_delay_ms)
                } else {
                    compute_backoff(config, attempt)
                };

                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
    }

    unreachable!("retry loop should have returned")
}

// ---------------------------------------------------------------------------
// Pre-built configs
// ---------------------------------------------------------------------------

/// Retry config for LLM API calls.
/// 3 attempts, 1s initial delay, up to 60s, 20% jitter.
pub fn llm_retry_config() -> RetryConfig {
    RetryConfig {
        max_attempts: 3,
        min_delay_ms: 1_000,
        max_delay_ms: 60_000,
        jitter: 0.2,
    }
}

/// Retry config for network operations (webhooks, fetches).
/// 3 attempts, 500ms initial delay, up to 30s, 10% jitter.
pub fn network_retry_config() -> RetryConfig {
    RetryConfig {
        max_attempts: 3,
        min_delay_ms: 500,
        max_delay_ms: 30_000,
        jitter: 0.1,
    }
}

/// Retry config for channel message delivery.
/// 3 attempts, 400ms initial delay, up to 15s, 10% jitter.
pub fn channel_retry_config() -> RetryConfig {
    RetryConfig {
        max_attempts: 3,
        min_delay_ms: 400,
        max_delay_ms: 15_000,
        jitter: 0.1,
    }
}

/// Retry config for WASM compilation/execution.
/// 2 attempts, 200ms initial delay, up to 5s, no jitter.
pub fn wasm_retry_config() -> RetryConfig {
    RetryConfig {
        max_attempts: 2,
        min_delay_ms: 200,
        max_delay_ms: 5_000,
        jitter: 0.0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.min_delay_ms, 300);
        assert_eq!(config.max_delay_ms, 30_000);
        assert!((config.jitter - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_backoff_exponential() {
        let config = RetryConfig {
            max_attempts: 5,
            min_delay_ms: 100,
            max_delay_ms: 100_000,
            jitter: 0.0,
        };
        assert_eq!(compute_backoff(&config, 0), 100);
        assert_eq!(compute_backoff(&config, 1), 200);
        assert_eq!(compute_backoff(&config, 2), 400);
        assert_eq!(compute_backoff(&config, 3), 800);
    }

    #[test]
    fn test_compute_backoff_capped() {
        let config = RetryConfig {
            max_attempts: 10,
            min_delay_ms: 1_000,
            max_delay_ms: 5_000,
            jitter: 0.0,
        };
        assert_eq!(compute_backoff(&config, 0), 1_000);
        assert_eq!(compute_backoff(&config, 1), 2_000);
        assert_eq!(compute_backoff(&config, 2), 4_000);
        assert_eq!(compute_backoff(&config, 3), 5_000); // capped
        assert_eq!(compute_backoff(&config, 10), 5_000);
    }

    #[test]
    fn test_compute_backoff_with_jitter() {
        let config = RetryConfig {
            max_attempts: 3,
            min_delay_ms: 1000,
            max_delay_ms: 60_000,
            jitter: 0.2,
        };
        let delay = compute_backoff(&config, 0);
        // With 20% jitter, delay should be between 1000 and 1200
        assert!(delay >= 1000);
        assert!(delay <= 1200);
    }

    #[test]
    fn test_retry_success_first_try() {
        let config = RetryConfig {
            max_attempts: 3,
            min_delay_ms: 1,
            max_delay_ms: 10,
            jitter: 0.0,
        };

        let outcome = retry_sync(
            &config,
            || Ok::<&str, &str>("hello"),
            |_| true,
            |_: &&str| None,
        );

        match outcome {
            RetryOutcome::Success { result, attempts } => {
                assert_eq!(result, "hello");
                assert_eq!(attempts, 1);
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn test_retry_success_after_failures() {
        let config = RetryConfig {
            max_attempts: 5,
            min_delay_ms: 1,
            max_delay_ms: 10,
            jitter: 0.0,
        };

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let outcome = retry_sync(
            &config,
            move || {
                let n = counter_clone.fetch_add(1, Ordering::SeqCst);
                if n < 2 { Err("not yet") } else { Ok("finally") }
            },
            |_| true,
            |_: &&str| None,
        );

        match outcome {
            RetryOutcome::Success { result, attempts } => {
                assert_eq!(result, "finally");
                assert_eq!(attempts, 3);
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn test_retry_exhausted() {
        let config = RetryConfig {
            max_attempts: 3,
            min_delay_ms: 1,
            max_delay_ms: 10,
            jitter: 0.0,
        };

        let outcome = retry_sync(
            &config,
            || Err::<(), &str>("always fails"),
            |_| true,
            |_: &&str| None,
        );

        match outcome {
            RetryOutcome::Exhausted {
                last_error,
                attempts,
            } => {
                assert_eq!(last_error, "always fails");
                assert_eq!(attempts, 3);
            }
            _ => panic!("expected exhausted"),
        }
    }

    #[test]
    fn test_retry_non_retryable_error() {
        let config = RetryConfig {
            max_attempts: 5,
            min_delay_ms: 1,
            max_delay_ms: 10,
            jitter: 0.0,
        };

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let outcome = retry_sync(
            &config,
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err::<(), &str>("fatal error")
            },
            |_| false, // never retry
            |_: &&str| None,
        );

        match outcome {
            RetryOutcome::Exhausted {
                last_error,
                attempts,
            } => {
                assert_eq!(last_error, "fatal error");
                assert_eq!(attempts, 1);
            }
            _ => panic!("expected exhausted"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_retry_with_hint_delay() {
        let config = RetryConfig {
            max_attempts: 3,
            min_delay_ms: 10_000, // large base
            max_delay_ms: 60_000,
            jitter: 0.0,
        };

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        let start = std::time::Instant::now();

        let outcome = retry_sync(
            &config,
            move || {
                let n = counter_clone.fetch_add(1, Ordering::SeqCst);
                if n < 1 { Err("transient") } else { Ok("ok") }
            },
            |_| true,
            |_: &&str| Some(1), // 1ms hint overrides 10s base
        );

        let elapsed = start.elapsed();
        match outcome {
            RetryOutcome::Success { result, attempts } => {
                assert_eq!(result, "ok");
                assert_eq!(attempts, 2);
                assert!(
                    elapsed.as_millis() < 5_000,
                    "hint should override base delay"
                );
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn test_llm_retry_config() {
        let config = llm_retry_config();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.min_delay_ms, 1_000);
    }

    #[test]
    fn test_channel_retry_config() {
        let config = channel_retry_config();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.min_delay_ms, 400);
    }

    #[test]
    fn test_network_retry_config() {
        let config = network_retry_config();
        assert_eq!(config.min_delay_ms, 500);
    }

    #[test]
    fn test_wasm_retry_config() {
        let config = wasm_retry_config();
        assert_eq!(config.max_attempts, 2);
        assert_eq!(config.min_delay_ms, 200);
        assert!((config.jitter - 0.0).abs() < f64::EPSILON);
    }
}
