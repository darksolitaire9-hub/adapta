#![cfg_attr(not(feature = "std"), no_std)]

pub mod adajepa;
pub mod edgeworth;

/// Core adaptation strategy trait for closed-loop error correction and retry tuning.
pub trait AdaptationStrategy<E, O> {
    /// Ingests an observed error, latency, or target signal into the active window.
    fn observe(&mut self, signal: E);
    
    /// Calculates the next adaptation output or backoff margin multiplier.
    fn adapt(&self) -> Option<O>;
    
    /// Current number of valid samples observed in the active window.
    fn sample_count(&self) -> usize;
}
