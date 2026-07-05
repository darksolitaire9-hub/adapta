use crate::AdaptationStrategy;

/// An adaptive retry margin tuner based on Edgeworth expansions of higher-order central moments
/// (skewness and excess kurtosis) of failure latency distributions.
///
/// Guardrail: Enforces a minimum sample threshold of N >= 15 before applying statistical correction
/// to prevent numerical variance and float division instability (NaN/Inf when sigma -> 0).
#[derive(Debug, Clone)]
pub struct EdgeworthBackoff {
    buffer: [f64; 32],
    head: usize,
    count: usize,
    base_backoff_ms: f64,
    c_skew: f64,
    c_kurt: f64,
}

impl EdgeworthBackoff {
    /// Creates a new EdgeworthBackoff tuner with specified base delay and tuning weights.
    pub fn new(base_backoff_ms: f64, c_skew: f64, c_kurt: f64) -> Self {
        Self {
            buffer: [0.0; 32],
            head: 0,
            count: 0,
            base_backoff_ms,
            c_skew,
            c_kurt,
        }
    }

    /// Computes exact sample mean, second, third, and fourth central moments.
    /// Returns (mean, m2, m3, m4).
    pub fn compute_moments(&self) -> (f64, f64, f64, f64) {
        if self.count == 0 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let n = self.count as f64;
        let mut sum = 0.0;
        for i in 0..self.count {
            sum += self.buffer[i];
        }
        let mean = sum / n;

        let mut m2 = 0.0;
        let mut m3 = 0.0;
        let mut m4 = 0.0;
        for i in 0..self.count {
            let diff = self.buffer[i] - mean;
            let diff2 = diff * diff;
            m2 += diff2;
            m3 += diff2 * diff;
            m4 += diff2 * diff2;
        }
        (mean, m2 / n, m3 / n, m4 / n)
    }

    /// Calculates Fisher-Pearson skewness (g1) and excess kurtosis (g2).
    /// Returns None if sample count < 15 or variance m2 < 1e-12.
    pub fn compute_skew_kurt(&self) -> Option<(f64, f64)> {
        if self.count < 15 {
            return None;
        }
        let (_, m2, m3, m4) = self.compute_moments();
        if m2 < 1e-12 {
            return None;
        }
        
        #[cfg(feature = "std")]
        let (m2_pow_1_5, m2_sq) = (m2 * m2.sqrt(), m2 * m2);

        #[cfg(not(feature = "std"))]
        let (m2_pow_1_5, m2_sq) = {
            let mut s = m2;
            for _ in 0..15 {
                if s == 0.0 { break; }
                s = 0.5 * (s + m2 / s);
            }
            (m2 * s, m2 * m2)
        };

        let g1 = m3 / m2_pow_1_5;
        let g2 = (m4 / m2_sq) - 3.0;
        Some((g1, g2))
    }
}

impl AdaptationStrategy<f64, f64> for EdgeworthBackoff {
    fn observe(&mut self, signal: f64) {
        self.buffer[self.head] = signal;
        self.head = (self.head + 1) % self.buffer.len();
        if self.count < self.buffer.len() {
            self.count += 1;
        }
    }

    fn adapt(&self) -> Option<f64> {
        match self.compute_skew_kurt() {
            Some((g1, g2)) => {
                // Margin multiplier M = 1.0 + c_skew * max(0.0, g1) + c_kurt * max(0.0, g2)
                let g1_pos = if g1 > 0.0 { g1 } else { 0.0 };
                let g2_pos = if g2 > 0.0 { g2 } else { 0.0 };
                let multiplier = 1.0 + self.c_skew * g1_pos + self.c_kurt * g2_pos;
                Some(self.base_backoff_ms * multiplier)
            }
            None => {
                // Guardrail fallback: return canonical base backoff without higher-order expansion
                Some(self.base_backoff_ms)
            }
        }
    }

    fn sample_count(&self) -> usize {
        self.count
    }
}
