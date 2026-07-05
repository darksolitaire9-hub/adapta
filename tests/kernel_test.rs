use adapta::edgeworth::EdgeworthBackoff;
use adapta::adajepa::AdaJepaCorrector;
use adapta::AdaptationStrategy;

#[test]
fn test_edgeworth_guardrail_below_15_samples() {
    let mut tuner = EdgeworthBackoff::new(100.0, 0.1, 0.05);
    
    // Feed 14 samples (below N=15 threshold)
    for _ in 0..14 {
        tuner.observe(50.0);
    }
    
    assert_eq!(tuner.sample_count(), 14);
    assert!(tuner.compute_skew_kurt().is_none(), "Must return None below N=15 threshold");
    
    // Adapt must fall back to exact base backoff without higher-order margin multiplier
    assert_eq!(tuner.adapt(), Some(100.0));
}

#[test]
fn test_edgeworth_reference_vector_precision() {
    let mut tuner = EdgeworthBackoff::new(100.0, 0.1, 0.05);
    
    // Known test vector of 15 elements
    // Python reference: mean=16.2, m2=113.62666667, g1=2.21894643, g2=4.00433476
    let test_vector = [
        10.0, 12.0, 15.0, 11.0, 10.0, 25.0, 30.0, 12.0, 11.0, 10.0, 10.0, 11.0, 14.0, 50.0, 12.0
    ];
    
    for &val in &test_vector {
        tuner.observe(val);
    }
    
    assert_eq!(tuner.sample_count(), 15);
    let (mean, m2, m3, m4) = tuner.compute_moments();
    assert!((mean - 16.2).abs() < 1e-6, "Mean mismatch: expected 16.2, got {}", mean);
    assert!((m2 - 113.62666667).abs() < 1e-6, "m2 mismatch: expected 113.62666667, got {}", m2);
    assert!(m3 > 0.0);
    assert!(m4 > 0.0);
    
    let (g1, g2) = tuner.compute_skew_kurt().expect("Must compute skew/kurt at N=15");
    assert!((g1 - 2.21894643).abs() < 1e-6, "Skewness g1 mismatch: expected 2.21894643, got {}", g1);
    assert!((g2 - 4.00433476).abs() < 1e-6, "Kurtosis g2 mismatch: expected 4.00433476, got {}", g2);
    
    // Calculate expected margin multiplier: 1.0 + 0.1 * 2.21894643 + 0.05 * 4.00433476 = 1.422111381
    let expected_multiplier = 1.0 + 0.1 * g1 + 0.05 * g2;
    let expected_backoff = 100.0 * expected_multiplier;
    let adapted = tuner.adapt().unwrap();
    assert!((adapted - expected_backoff).abs() < 1e-6, "Adapted backoff mismatch: expected {}, got {}", expected_backoff, adapted);
}

#[test]
fn test_edgeworth_zero_variance_guardrail() {
    let mut tuner = EdgeworthBackoff::new(100.0, 0.1, 0.05);
    
    // Feed 20 identical samples (variance m2 = 0.0)
    for _ in 0..20 {
        tuner.observe(25.0);
    }
    
    assert_eq!(tuner.sample_count(), 20);
    assert!(tuner.compute_skew_kurt().is_none(), "Must return None when variance m2 -> 0 to avoid NaN/Inf");
    assert_eq!(tuner.adapt(), Some(100.0), "Must fall back to base backoff when variance is zero");
}

#[test]
fn test_adajepa_1_step_drift_correction() {
    let mut corrector = AdaJepaCorrector::new(100.0, 0.1);
    assert_eq!(corrector.sample_count(), 0);
    assert_eq!(corrector.adapt(), Some(100.0));
    
    // Observe target 110.0 -> error = 10.0 -> baseline <- 100.0 + 0.1 * 10.0 = 101.0
    corrector.observe(110.0);
    assert_eq!(corrector.sample_count(), 1);
    assert!((corrector.adapt().unwrap() - 101.0).abs() < 1e-9);
    
    // Observe target 105.0 -> error = 4.0 -> baseline <- 101.0 + 0.1 * 4.0 = 101.4
    corrector.observe(105.0);
    assert_eq!(corrector.sample_count(), 2);
    assert!((corrector.adapt().unwrap() - 101.4).abs() < 1e-9);
}

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_edgeworth_never_nan_or_inf(
        signals in proptest::collection::vec(-1e6f64..1e6f64, 1..50),
        base in 1.0f64..1000.0f64,
        c_skew in 0.0f64..1.0f64,
        c_kurt in 0.0f64..1.0f64
    ) {
        let mut tuner = EdgeworthBackoff::new(base, c_skew, c_kurt);
        for &sig in &signals {
            tuner.observe(sig);
            if let Some(val) = tuner.adapt() {
                prop_assert!(!val.is_nan(), "adapt() returned NaN for signals len {}", signals.len());
                prop_assert!(!val.is_infinite(), "adapt() returned Inf for signals len {}", signals.len());
                prop_assert!(val > 0.0, "adapt() backoff must be positive");
            }
            if let Some((g1, g2)) = tuner.compute_skew_kurt() {
                prop_assert!(!g1.is_nan(), "skewness g1 was NaN");
                prop_assert!(!g1.is_infinite(), "skewness g1 was Inf");
                prop_assert!(!g2.is_nan(), "kurtosis g2 was NaN");
                prop_assert!(!g2.is_infinite(), "kurtosis g2 was Inf");
            }
        }
    }

    #[test]
    fn prop_edgeworth_strict_fallback_below_15(
        signals in proptest::collection::vec(-1e5f64..1e5f64, 1..14),
        base in 10.0f64..500.0f64
    ) {
        let mut tuner = EdgeworthBackoff::new(base, 0.2, 0.1);
        for &sig in &signals {
            tuner.observe(sig);
            prop_assert!(tuner.compute_skew_kurt().is_none(), "Must not compute skew/kurt below N=15");
            prop_assert_eq!(tuner.adapt(), Some(base), "Must return exact base backoff below N=15");
        }
    }

    #[test]
    fn prop_adajepa_drift_bounded(
        signals in proptest::collection::vec(-1000.0f64..1000.0f64, 1..100),
        initial in -500.0f64..500.0f64,
        eta in 0.001f64..0.5f64
    ) {
        let mut corrector = AdaJepaCorrector::new(initial, eta);
        for &sig in &signals {
            corrector.observe(sig);
            let adapted = corrector.adapt().expect("AdaJEPA must always produce adapted baseline");
            prop_assert!(!adapted.is_nan(), "AdaJEPA adapted baseline returned NaN");
            prop_assert!(!adapted.is_infinite(), "AdaJEPA adapted baseline returned Inf");
        }
    }
}

