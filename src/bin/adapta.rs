use adapta::adajepa::AdaJepaCorrector;
use adapta::edgeworth::EdgeworthBackoff;
use adapta::AdaptationStrategy;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("adapta v0.0.2 CLI - Zero-latency closed-loop adaptation kernel");
        eprintln!("Usage:");
        eprintln!("  adapta edgeworth <base_ms> <c_skew> <c_kurt> <latencies_comma_separated>");
        eprintln!("  adapta adajepa <initial_baseline> <eta> <observed_signal>");
        eprintln!("Example:");
        eprintln!("  adapta edgeworth 100.0 0.1 0.05 10.0,12.0,15.0,11.0,10.0,25.0,30.0,12.0,11.0,10.0,10.0,11.0,14.0,50.0,12.0");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "edgeworth" => {
            if args.len() < 6 {
                eprintln!("Error: edgeworth requires <base_ms> <c_skew> <c_kurt> <latencies>");
                std::process::exit(1);
            }
            let base_ms: f64 = args[2].parse().unwrap_or(100.0);
            let c_skew: f64 = args[3].parse().unwrap_or(0.1);
            let c_kurt: f64 = args[4].parse().unwrap_or(0.05);
            let mut tuner = EdgeworthBackoff::new(base_ms, c_skew, c_kurt);

            for val_str in args[5].split(',') {
                if let Ok(val) = val_str.trim().parse::<f64>() {
                    tuner.observe(val);
                }
            }

            let (mean, _, _, _) = tuner.compute_moments();
            let count = tuner.sample_count();
            match tuner.adapt() {
                Some(adjusted) => {
                    println!(
                        "count={} mean={:.2} adjusted_timeout_ms={:.2}",
                        count, mean, adjusted
                    );
                }
                None => {
                    println!(
                        "count={} mean={:.2} adjusted_timeout_ms={:.2} (guardrail fallback)",
                        count, mean, base_ms
                    );
                }
            }
        }
        "adajepa" => {
            if args.len() < 5 {
                eprintln!("Error: adajepa requires <initial_baseline> <eta> <observed_signal>");
                std::process::exit(1);
            }
            let initial: f64 = args[2].parse().unwrap_or(10.0);
            let eta: f64 = args[3].parse().unwrap_or(0.1);
            let target: f64 = args[4].parse().unwrap_or(10.0);

            let mut corrector = AdaJepaCorrector::new(initial, eta);
            corrector.observe(target);
            if let Some(new_base) = corrector.adapt() {
                println!(
                    "initial={:.2} target={:.2} new_baseline={:.2}",
                    initial, target, new_base
                );
            }
        }
        _ => {
            eprintln!("Unknown subcommand: {}", args[1]);
            std::process::exit(1);
        }
    }
}
