use std::collections::HashSet;

use super::*;

fn make_counts(n1: &[&str], n2: &[&str], big_n1: usize, big_n2: usize) -> TokenCounts {
    TokenCounts {
        distinct_operators: n1.iter().map(|s| s.to_string()).collect(),
        distinct_operands: n2.iter().map(|s| s.to_string()).collect(),
        total_operators: big_n1,
        total_operands: big_n2,
    }
}

#[test]
fn empty_returns_none() {
    let counts = TokenCounts {
        distinct_operators: HashSet::new(),
        distinct_operands: HashSet::new(),
        total_operators: 0,
        total_operands: 0,
    };
    assert!(compute(&counts).is_none());
}

#[test]
fn basic_computation() {
    // η₁=2, η₂=3, N₁=5, N₂=7
    let counts = make_counts(&["if", "+"], &["x", "y", "1"], 5, 7);
    let m = compute(&counts).unwrap();

    assert_eq!(m.distinct_operators, 2);
    assert_eq!(m.distinct_operands, 3);
    assert_eq!(m.total_operators, 5);
    assert_eq!(m.total_operands, 7);
    assert_eq!(m.vocabulary, 5); // 2 + 3
    assert_eq!(m.length, 12); // 5 + 7

    // V = 12 × log₂(5) ≈ 12 × 2.3219 ≈ 27.863
    assert!((m.volume - 27.863).abs() < 0.1);

    // D = (2/2) × (7/3) ≈ 2.333
    assert!((m.difficulty - 2.333).abs() < 0.01);

    // E = D × V ≈ 65.014
    assert!((m.effort - 65.01).abs() < 0.1);

    // B = V / 3000 ≈ 0.00929
    assert!((m.bugs - 0.00929).abs() < 0.001);

    // T = E / 18 ≈ 3.612
    assert!((m.time - 3.612).abs() < 0.1);
}

#[test]
fn zero_operands_returns_none() {
    // Only operators, no operands → not meaningful code
    let counts = make_counts(&["if"], &[], 3, 0);
    assert!(compute(&counts).is_none());
}

#[test]
fn zero_operators_returns_none() {
    // Only operands, no operators → not meaningful code
    let counts = make_counts(&[], &["x", "y"], 0, 5);
    assert!(compute(&counts).is_none());
}

#[test]
fn single_operator_single_operand() {
    let counts = make_counts(&["="], &["x"], 1, 1);
    let m = compute(&counts).unwrap();

    assert_eq!(m.vocabulary, 2);
    assert_eq!(m.length, 2);
    // V = 2 × log₂(2) = 2.0
    assert!((m.volume - 2.0).abs() < 0.001);
}
