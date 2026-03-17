//! Verify that `run_utm_on(tape)` and `tm::run(&utm_tm, &tape)` produce
//! identical results for several test TMs.

use std::collections::HashMap;
use utmmmmm::tm::{self, Dir, Outcome, TuringMachine};
use utmmmmm::utm::{self, UtmSym};

// ---- Test TMs (must fit state_bits=2, symbol_bits=1) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum W1State { Start, Accept, Reject }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum W1Sym { Blank, One }

/// Writes 1 and halts. 1 step.
fn write1_tm() -> TuringMachine<W1State, W1Sym> {
    let mut t = HashMap::new();
    t.insert(
        (W1State::Start, W1Sym::Blank),
        (W1State::Accept, W1Sym::One, Dir::Right),
    );
    TuringMachine {
        initial: W1State::Start,
        accept: W1State::Accept,
        reject: W1State::Reject,
        blank: W1Sym::Blank,
        transitions: t,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum W11State { Start, Second, Accept, Reject }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum W11Sym { Blank, One }

/// Writes 1, moves right, writes 1, halts. 2 steps.
fn write11_tm() -> TuringMachine<W11State, W11Sym> {
    let mut t = HashMap::new();
    t.insert(
        (W11State::Start, W11Sym::Blank),
        (W11State::Second, W11Sym::One, Dir::Right),
    );
    t.insert(
        (W11State::Second, W11Sym::Blank),
        (W11State::Accept, W11Sym::One, Dir::Right),
    );
    TuringMachine {
        initial: W11State::Start,
        accept: W11State::Accept,
        reject: W11State::Reject,
        blank: W11Sym::Blank,
        transitions: t,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RejState { Start, Accept, Reject }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RejSym { Blank, One }

/// Immediately rejects (no transition from Start+Blank → reject state). 0 steps.
fn reject_tm() -> TuringMachine<RejState, RejSym> {
    let mut t = HashMap::new();
    t.insert(
        (RejState::Start, RejSym::Blank),
        (RejState::Reject, RejSym::Blank, Dir::Right),
    );
    TuringMachine {
        initial: RejState::Start,
        accept: RejState::Accept,
        reject: RejState::Reject,
        blank: RejSym::Blank,
        transitions: t,
    }
}

fn strip_trailing<A: Eq + Clone>(tape: &[A], blank: &A) -> Vec<A> {
    let mut v = tape.to_vec();
    while v.last() == Some(blank) { v.pop(); }
    v
}

fn compare<S, A>(
    name: &str,
    tm_under_test: &TuringMachine<S, A>,
    input: &[A],
    max_steps: usize,
)
where
    S: Eq + std::hash::Hash + Clone + std::fmt::Debug,
    A: Eq + std::hash::Hash + Clone + std::fmt::Debug,
{
    let encoded = utm::encode(tm_under_test, input);
    println!("  [{name}] encoded tape: {} symbols", encoded.len());

    // --- Path A: software interpreter ---
    let (interp_tape, interp_accepted) = utm::run_utm_on(&encoded, max_steps)
        .unwrap_or_else(|| panic!("[{name}] run_utm_on did not halt in {max_steps} steps"));
    let interp_decoded = utm::decode(tm_under_test, &interp_tape, interp_accepted);
    println!("  [{name}] run_utm_on: outcome={:?}, tape={:?}",
        interp_decoded.outcome,
        strip_trailing(&interp_decoded.tape, &tm_under_test.blank));

    // --- Path B: actual UTM TM ---
    let utm_tm = utm::build_utm_tm();
    let hw_result = tm::run(&utm_tm, &encoded, max_steps)
        .unwrap_or_else(|| panic!("[{name}] tm::run(utm_tm, ...) did not halt in {max_steps} steps"));
    let hw_decoded = utm::decode(
        tm_under_test,
        &hw_result.tape,
        hw_result.outcome == Outcome::Accept,
    );
    println!("  [{name}] tm::run(utm_tm): outcome={:?}, tape={:?}",
        hw_decoded.outcome,
        strip_trailing(&hw_decoded.tape, &tm_under_test.blank));

    // --- Compare ---
    assert_eq!(
        interp_decoded.outcome, hw_decoded.outcome,
        "[{name}] outcomes differ"
    );
    assert_eq!(
        strip_trailing(&interp_decoded.tape, &tm_under_test.blank),
        strip_trailing(&hw_decoded.tape, &tm_under_test.blank),
        "[{name}] decoded tapes differ"
    );

    println!("  [{name}] OK ✓");
}

fn main() {
    println!("Verifying run_utm_on ≡ tm::run(utm_tm, ...) ...\n");

    let limit = 10_000_000;

    println!("1. write1 (1 step, accepts):");
    compare("write1", &write1_tm(), &[], limit);

    println!("\n2. write11 (2 steps, accepts):");
    compare("write11", &write11_tm(), &[], limit);

    println!("\n3. reject (1 step, rejects):");
    compare("reject", &reject_tm(), &[], limit);

    println!("\nAll equivalent. ✓");
}
