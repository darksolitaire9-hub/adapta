use crate::AdaptationStrategy;

/// A 1-step closed-loop test-time adaptation (TTA) drift corrector inspired by AdaJEPA (arXiv:2606.32026).
///
/// Monitors prediction error Delta_t = target - baseline across incoming signals and applies
/// online gradient corrections baseline <- baseline + eta * Delta_t to realign evaluation
/// or world-model baselines under benign distribution shift.
#[derive(Debug, Clone)]
pub struct AdaJepaCorrector {
    baseline: f64,
    eta: f64,
    count: usize,
}

impl AdaJepaCorrector {
    /// Creates a new AdaJEPA corrector with specified initial baseline and learning rate eta.
    pub fn new(initial_baseline: f64, eta: f64) -> Self {
        Self {
            baseline: initial_baseline,
            eta,
            count: 0,
        }
    }

    /// Returns the current adapted baseline prediction.
    pub fn current_baseline(&self) -> f64 {
        self.baseline
    }
}

impl AdaptationStrategy<f64, f64> for AdaJepaCorrector {
    fn observe(&mut self, target_signal: f64) {
        let error = target_signal - self.baseline;
        self.baseline += self.eta * error;
        self.count += 1;
    }

    fn adapt(&self) -> Option<f64> {
        Some(self.baseline)
    }

    fn sample_count(&self) -> usize {
        self.count
    }
}
