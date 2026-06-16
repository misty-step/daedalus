//! Parity oracle for pycompat::round_half_even — it must equal CPython's
//! `round(x, n)` bit-for-bit across half-point-prone values, since the grader,
//! cost aggregates, and Pareto dominance all depend on it. Skips when python3
//! is unavailable.

use std::process::Command;

use daedalus_core::pycompat::round_half_even;

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn round_half_even_matches_python_round() {
    if !python_available() {
        eprintln!("skipping pycompat parity: python3 not available");
        return;
    }

    // (value, ndigits) pairs that stress half-to-even rounding and the decimal
    // half-points where the naive multiply trick diverges.
    let mut pairs: Vec<(f64, u32)> = vec![
        (0.5, 0),
        (1.5, 0),
        (2.5, 0),
        (3.5, 0),
        (-0.5, 0),
        (-2.5, 0),
        (0.125, 2),
        (0.135, 2),
        (0.145, 2),
        (0.155, 2),
        (0.105, 2),
        (2.675, 2), // classic float-repr gotcha: 2.675 -> 2.67
        (0.0000005, 6),
        (0.0000015, 6),
        (0.0000025, 6),
        (0.00000049999, 6),
        (1.0, 4),
        (0.0, 4),
    ];
    // reward/recall fractions k/n at 4 and cost-per-trial-like values at 6
    for n in 1..=20u32 {
        for k in 0..=n {
            pairs.push((k as f64 / n as f64, 4));
            pairs.push((k as f64 / n as f64 / 7.0, 6));
        }
    }
    // rounded-cost / trials patterns (report.cost_per_trial divides a 4-dp cost)
    for cost_milli in 1..=50i64 {
        let cost = round_half_even(cost_milli as f64 / 10000.0, 4);
        for trials in 1..=8i64 {
            pairs.push((cost / trials as f64, 6));
        }
    }

    let payload: Vec<(f64, u32)> = pairs.clone();
    let input = serde_json::to_string(&payload).unwrap();

    let out = Command::new("python3")
        .arg("-c")
        .arg(
            "import sys, json; \
             pairs = json.load(sys.stdin); \
             print(json.dumps([round(x, n) for x, n in pairs]))",
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .expect("run python round");
    assert!(
        out.status.success(),
        "python round failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let py: Vec<f64> = serde_json::from_slice(&out.stdout).expect("python emitted json");

    assert_eq!(py.len(), pairs.len());
    let mut mismatches = Vec::new();
    for (i, (x, n)) in pairs.iter().enumerate() {
        let rust = round_half_even(*x, *n);
        // exact equality (both are correctly-rounded to the same decimal -> same f64)
        if rust.to_bits() != py[i].to_bits() && !(rust.is_nan() && py[i].is_nan()) {
            mismatches.push(format!("round({x}, {n}): py={} rust={rust}", py[i]));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} divergences from Python round():\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
