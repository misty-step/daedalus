//! Parity oracle: `PyRandom` must reproduce CPython's `random.Random` exactly —
//! both `getrandbits` and `shuffle`, since the search loop and seeder depend on
//! the seeded trajectory. Skips when python3 is unavailable.

use std::process::Command;

use threshold_core::pyrandom::PyRandom;

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn shuffle_matches_cpython() {
    if !python_available() {
        eprintln!("skipping pyrandom parity: python3 not available");
        return;
    }
    for &seed in &[0u64, 1, 2, 42, 12_345, 2_000_000_000] {
        for &size in &[2usize, 3, 5, 6, 7, 10, 16, 25, 50] {
            let out = Command::new("python3")
                .arg("-c")
                .arg(format!(
                    "import random, json; r = random.Random({seed}); \
                     l = list(range({size})); r.shuffle(l); print(json.dumps(l))"
                ))
                .output()
                .expect("run python shuffle");
            assert!(out.status.success(), "python shuffle failed");
            let py: Vec<usize> = serde_json::from_slice(&out.stdout).expect("json");
            let mut rust: Vec<usize> = (0..size).collect();
            PyRandom::new(seed).shuffle(&mut rust);
            assert_eq!(py, rust, "shuffle differs at seed={seed} size={size}");
        }
    }
}

#[test]
fn getrandbits_matches_cpython() {
    if !python_available() {
        return;
    }
    let ks = [1u32, 2, 3, 5, 8, 13, 16, 31, 32, 33, 40, 53, 64];
    let ks_py = ks
        .iter()
        .map(|k| k.to_string())
        .collect::<Vec<_>>()
        .join(",");
    for &seed in &[0u64, 7, 99, 123_456_789] {
        let out = Command::new("python3")
            .arg("-c")
            .arg(format!(
                "import random, json; r = random.Random({seed}); \
                 print(json.dumps([r.getrandbits(k) for k in [{ks_py}]]))"
            ))
            .output()
            .expect("run python getrandbits");
        assert!(out.status.success(), "python getrandbits failed");
        let py: Vec<u64> = serde_json::from_slice(&out.stdout).expect("json");
        let mut rng = PyRandom::new(seed);
        let rust: Vec<u64> = ks.iter().map(|&k| rng.getrandbits(k)).collect();
        assert_eq!(py, rust, "getrandbits differs at seed={seed}");
    }
}
