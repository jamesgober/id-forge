// Randomness checks for the random ID schemes: independent processes
// started together must not share IDs, and the random bits must look
// uniform.

#![cfg(all(feature = "uuid", feature = "ulid", feature = "nanoid"))]

use std::collections::HashSet;
use std::process::{Command, Stdio};

use id_forge::nanoid;
use id_forge::ulid::Ulid;
use id_forge::uuid::Uuid;

const CHILD_ENV: &str = "ID_FORGE_RANDOMNESS_CHILD";
const IDS_PER_CHILD: usize = 64;

/// Child mode: print a batch of IDs from a fresh process and exit.
/// Runs only when the parent test below re-executes this binary.
#[test]
fn child_emit_ids() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    // Leading newline: the harness prints "test ... " before the
    // captured output on the same line.
    let mut out = String::from('\n');
    for _ in 0..IDS_PER_CHILD {
        out.push_str(&format!("uuid {}\n", Uuid::v4()));
        out.push_str(&format!("ulid {}\n", Ulid::new()));
        out.push_str(&format!("nano {}\n", nanoid::generate()));
    }
    print!("{out}");
}

fn spawn_child() -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "child_emit_ids",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_ENV, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn ids_from(child: std::process::Child) -> Vec<String> {
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let ids: Vec<String> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("uuid ") || l.starts_with("ulid ") || l.starts_with("nano "))
        .map(|l| l[5..].to_string())
        .collect();
    assert_eq!(ids.len(), IDS_PER_CHILD * 3, "child printed {ids:?}");
    ids
}

#[test]
fn processes_started_together_produce_different_ids() {
    // Start several processes back to back so they share the same clock
    // window, then require every ID across all of them to be distinct.
    let children: Vec<_> = (0..4).map(|_| spawn_child()).collect();
    let mut all = HashSet::new();
    let mut total = 0;
    for child in children {
        for id in ids_from(child) {
            total += 1;
            assert!(all.insert(id.clone()), "ID {id} appeared in two processes");
        }
    }
    assert_eq!(total, 4 * IDS_PER_CHILD * 3);
}

/// Each of the 122 random bits of a v4 UUID must be set close to half
/// the time. With 20 000 samples a fair bit lands in 10 000 +/- 300
/// (about 4.2 standard deviations); a stuck or biased bit falls far
/// outside.
#[test]
fn uuid_v4_bits_are_balanced() {
    const N: u32 = 20_000;
    let mut ones = [0u32; 128];
    for _ in 0..N {
        let bytes = *Uuid::v4().as_bytes();
        for (bit, count) in ones.iter_mut().enumerate() {
            if bytes[bit / 8] & (0x80 >> (bit % 8)) != 0 {
                *count += 1;
            }
        }
    }
    // Version nibble (bits 48..52) and variant bits (64, 65) are fixed.
    let fixed = |bit: usize| (48..52).contains(&bit) || bit == 64 || bit == 65;
    for (bit, &count) in ones.iter().enumerate() {
        if fixed(bit) {
            continue;
        }
        assert!(
            (N / 2 - 300..=N / 2 + 300).contains(&count),
            "bit {bit} set {count} times out of {N}"
        );
    }
}

/// The ULID random part (low 80 bits) must also be balanced. Each ULID
/// is drawn on a fresh thread so the monotonic increment never applies.
#[test]
fn ulid_random_bits_are_balanced() {
    const THREADS: u32 = 40;
    const PER: u32 = 250;
    const N: u32 = THREADS * PER;
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            std::thread::spawn(|| {
                (0..PER)
                    .map(|_| {
                        // A new thread per sample is too slow; instead
                        // sleep past the millisecond so each ID gets
                        // fresh random bits.
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        u128::from_be_bytes(*Ulid::new().as_bytes()) & ((1u128 << 80) - 1)
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut ones = [0u32; 80];
    for h in handles {
        for r in h.join().unwrap() {
            for (bit, count) in ones.iter_mut().enumerate() {
                if r >> bit & 1 == 1 {
                    *count += 1;
                }
            }
        }
    }
    // 10 000 samples: a fair bit lands in 5 000 +/- 300 (6 sigma; the
    // monotonic increment can touch the lowest bits on a rare
    // same-millisecond pair).
    for (bit, &count) in ones.iter().enumerate() {
        assert!(
            (N / 2 - 300..=N / 2 + 300).contains(&count),
            "ULID random bit {bit} set {count} times out of {N}"
        );
    }
}

/// NanoID characters must spread evenly over the 64-symbol alphabet.
/// Chi-square with 63 degrees of freedom: the 99.99th percentile is
/// about 117, so a bound of 130 almost never flakes on a fair source.
#[test]
fn nanoid_characters_are_uniform() {
    const IDS: usize = 20_000;
    let mut counts = std::collections::HashMap::new();
    let mut total = 0usize;
    for _ in 0..IDS {
        for c in nanoid::generate().chars() {
            *counts.entry(c).or_insert(0usize) += 1;
            total += 1;
        }
    }
    assert_eq!(counts.len(), 64, "alphabet coverage");
    let expected = total as f64 / 64.0;
    let chi2: f64 = counts
        .values()
        .map(|&o| {
            let d = o as f64 - expected;
            d * d / expected
        })
        .sum();
    assert!(
        chi2 < 130.0,
        "chi-square {chi2:.1} over 63 degrees of freedom"
    );
}
