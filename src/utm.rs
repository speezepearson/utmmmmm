use std::fmt::Debug;
use std::hash::Hash;

use crate::tm::{Dir, Outcome, RunResult, TuringMachine};

/// The UTM's tape alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UtmSym {
    Zero,
    One,
    LBracket,
    RBracket,
    Pipe,
    Semi,
    Hash,
    D,
    Blank,
}

/// Encode a TM and its input as a tape for the UTM.
///
/// Format:
/// ```text
/// <state_bits ones>;<symbol_bits ones>;<blank_sym binary>;<accept_state binary>;
/// [cs ca|ns na D d];...;[cs ca|ns na D d]
/// #[ss|aa][ss|aa]...
/// ```
///
/// - State/symbol fields are fixed-width binary.
/// - Reject state is always encoded as all-zeros (index 0).
/// - The header is self-describing: bit widths in unary, blank and accept in binary.
pub fn encode<S, A>(tm: &TuringMachine<S, A>, input: &[A]) -> Vec<UtmSym>
where
    S: Eq + Hash + Clone + Debug,
    A: Eq + Hash + Clone + Debug,
{
    let (states, symbols, params) = build_codec_tables(tm);

    let state_idx = |s: &S| -> usize { states.iter().position(|x| x == s).unwrap() };
    let sym_idx = |a: &A| -> usize { symbols.iter().position(|x| x == a).unwrap() };

    let mut tape: Vec<UtmSym> = Vec::new();

    // Header: state_bits in unary
    for _ in 0..params.state_bits {
        tape.push(UtmSym::One);
    }
    tape.push(UtmSym::Semi);

    // symbol_bits in unary
    for _ in 0..params.symbol_bits {
        tape.push(UtmSym::One);
    }
    tape.push(UtmSym::Semi);

    // blank symbol encoding in binary
    push_binary(&mut tape, sym_idx(&tm.blank), params.symbol_bits);
    tape.push(UtmSym::Semi);

    // accept state encoding in binary
    push_binary(&mut tape, state_idx(&tm.accept), params.state_bits);
    tape.push(UtmSym::Semi);

    // Transition table
    // Sort transitions for deterministic encoding
    let mut trans_entries: Vec<_> = tm.transitions.iter().collect();
    trans_entries.sort_by_key(|((s, a), _)| (state_idx(s), sym_idx(a)));

    let mut first = true;
    for ((s, a), (s2, a2, d)) in &trans_entries {
        if !first {
            tape.push(UtmSym::Semi);
        }
        first = false;
        tape.push(UtmSym::LBracket);
        push_binary(&mut tape, state_idx(s), params.state_bits);
        push_binary(&mut tape, sym_idx(a), params.symbol_bits);
        tape.push(UtmSym::Pipe);
        push_binary(&mut tape, state_idx(s2), params.state_bits);
        push_binary(&mut tape, sym_idx(a2), params.symbol_bits);
        tape.push(UtmSym::D);
        tape.push(match d {
            Dir::Left => UtmSym::Zero,
            Dir::Right => UtmSym::One,
        });
        tape.push(UtmSym::RBracket);
    }

    // Separator
    tape.push(UtmSym::Hash);

    // Data cells
    let initial_idx = state_idx(&tm.initial);
    if input.is_empty() {
        tape.push(UtmSym::LBracket);
        push_binary(&mut tape, initial_idx, params.state_bits);
        tape.push(UtmSym::Pipe);
        push_binary(&mut tape, sym_idx(&tm.blank), params.symbol_bits);
        tape.push(UtmSym::RBracket);
    } else {
        for (i, sym) in input.iter().enumerate() {
            tape.push(UtmSym::LBracket);
            if i == 0 {
                push_binary(&mut tape, initial_idx, params.state_bits);
            } else {
                push_binary(&mut tape, 0, params.state_bits);
            }
            tape.push(UtmSym::Pipe);
            push_binary(&mut tape, sym_idx(sym), params.symbol_bits);
            tape.push(UtmSym::RBracket);
        }
    }

    tape
}

/// Run the UTM on an encoded tape.
/// Returns (output_tape, accepted).
pub fn run_utm_on(encoded_tape: &[UtmSym], max_steps: usize) -> Option<(Vec<UtmSym>, bool)> {
    let tape = encoded_tape.to_vec();

    // Parse header
    let (state_bits, symbol_bits, blank_sym, accept_state, header_end) = parse_header(&tape);

    // Parse transitions
    let hash_pos = tape.iter().position(|s| matches!(s, UtmSym::Hash))
        .expect("no # in tape");
    let transitions = parse_transition_table(&tape[header_end..hash_pos], state_bits, symbol_bits);

    // Parse data cells
    let data_start = hash_pos + 1;
    let mut cells = parse_data_cells(&tape[data_start..], state_bits, symbol_bits);

    if cells.is_empty() {
        return Some((tape, false));
    }

    // Find head
    let mut head_pos = cells.iter().position(|(st, _)| *st != 0)
        .expect("no head cell found");
    let mut current_state = cells[head_pos].0;

    for _ in 0..max_steps {
        // Check halt
        if current_state == 0 {
            cells[head_pos].0 = 0;
            return Some((rebuild_tape(&tape[..=hash_pos], &cells, state_bits, symbol_bits), false));
        }
        if current_state == accept_state {
            cells[head_pos].0 = accept_state;
            return Some((rebuild_tape(&tape[..=hash_pos], &cells, state_bits, symbol_bits), true));
        }

        let current_symbol = cells[head_pos].1;

        // Find matching transition
        let tr = transitions.iter().find(|t| t.cur_state == current_state && t.cur_symbol == current_symbol);
        match tr {
            None => {
                // No transition → reject
                cells[head_pos].0 = 0; // reject
                return Some((rebuild_tape(&tape[..=hash_pos], &cells, state_bits, symbol_bits), false));
            }
            Some(tr) => {
                cells[head_pos].1 = tr.new_symbol;
                cells[head_pos].0 = 0; // clear head

                match tr.direction {
                    Dir::Left => {
                        if head_pos == 0 {
                            cells.insert(0, (0, blank_sym));
                            // head_pos stays 0
                        } else {
                            head_pos -= 1;
                        }
                    }
                    Dir::Right => {
                        head_pos += 1;
                        if head_pos >= cells.len() {
                            cells.push((0, blank_sym));
                        }
                    }
                }

                current_state = tr.new_state;
                cells[head_pos].0 = current_state;
            }
        }
    }

    None // didn't halt in time
}

/// Parsed header: (state_bits, symbol_bits, blank_sym, accept_state, header_end_pos)
fn parse_header(tape: &[UtmSym]) -> (usize, usize, u64, u64, usize) {
    let mut i = 0;

    // state_bits: count ones until ;
    let mut state_bits = 0;
    while i < tape.len() && matches!(tape[i], UtmSym::One) {
        state_bits += 1;
        i += 1;
    }
    assert!(matches!(tape[i], UtmSym::Semi));
    i += 1;

    // symbol_bits: count ones until ;
    let mut symbol_bits = 0;
    while i < tape.len() && matches!(tape[i], UtmSym::One) {
        symbol_bits += 1;
        i += 1;
    }
    assert!(matches!(tape[i], UtmSym::Semi));
    i += 1;

    // blank symbol: binary of symbol_bits width
    let blank_sym = read_binary_slice(&tape[i..], symbol_bits);
    i += symbol_bits;
    assert!(matches!(tape[i], UtmSym::Semi));
    i += 1;

    // accept state: binary of state_bits width
    let accept_state = read_binary_slice(&tape[i..], state_bits);
    i += state_bits;
    assert!(matches!(tape[i], UtmSym::Semi));
    i += 1;

    (state_bits, symbol_bits, blank_sym, accept_state, i)
}

#[derive(Debug, Clone)]
struct Transition {
    cur_state: u64,
    cur_symbol: u64,
    new_state: u64,
    new_symbol: u64,
    direction: Dir,
}

fn parse_transition_table(data: &[UtmSym], state_bits: usize, symbol_bits: usize) -> Vec<Transition> {
    let mut transitions = Vec::new();
    let mut i = 0;

    while i < data.len() {
        match data[i] {
            UtmSym::LBracket => {
                i += 1;
                let cur_state = read_binary_at(data, &mut i, state_bits);
                let cur_symbol = read_binary_at(data, &mut i, symbol_bits);
                assert!(matches!(data[i], UtmSym::Pipe));
                i += 1;
                let new_state = read_binary_at(data, &mut i, state_bits);
                let new_symbol = read_binary_at(data, &mut i, symbol_bits);
                assert!(matches!(data[i], UtmSym::D));
                i += 1;
                let direction = match data[i] {
                    UtmSym::Zero => Dir::Left,
                    UtmSym::One => Dir::Right,
                    _ => panic!("expected direction bit"),
                };
                i += 1;
                assert!(matches!(data[i], UtmSym::RBracket));
                i += 1;

                transitions.push(Transition {
                    cur_state,
                    cur_symbol,
                    new_state,
                    new_symbol,
                    direction,
                });
            }
            UtmSym::Semi => i += 1,
            _ => i += 1,
        }
    }

    transitions
}

fn parse_data_cells(data: &[UtmSym], state_bits: usize, symbol_bits: usize) -> Vec<(u64, u64)> {
    let mut cells = Vec::new();
    let mut i = 0;

    while i < data.len() {
        match data[i] {
            UtmSym::LBracket => {
                i += 1;
                let state_val = read_binary_at(data, &mut i, state_bits);
                assert!(matches!(data[i], UtmSym::Pipe));
                i += 1;
                let sym_val = read_binary_at(data, &mut i, symbol_bits);
                assert!(matches!(data[i], UtmSym::RBracket));
                i += 1;
                cells.push((state_val, sym_val));
            }
            _ => break, // end of data region
        }
    }

    cells
}

fn rebuild_tape(header_and_transitions: &[UtmSym], cells: &[(u64, u64)], state_bits: usize, symbol_bits: usize) -> Vec<UtmSym> {
    let mut tape = header_and_transitions.to_vec();
    for &(state_val, sym_val) in cells {
        tape.push(UtmSym::LBracket);
        push_binary_u64(&mut tape, state_val, state_bits);
        tape.push(UtmSym::Pipe);
        push_binary_u64(&mut tape, sym_val, symbol_bits);
        tape.push(UtmSym::RBracket);
    }
    tape
}

fn read_binary_slice(data: &[UtmSym], bits: usize) -> u64 {
    let mut val = 0u64;
    for i in 0..bits {
        val <<= 1;
        match data[i] {
            UtmSym::One => val |= 1,
            UtmSym::Zero => {}
            other => panic!("expected binary digit, got {:?}", other),
        }
    }
    val
}

fn read_binary_at(data: &[UtmSym], pos: &mut usize, bits: usize) -> u64 {
    let val = read_binary_slice(&data[*pos..], bits);
    *pos += bits;
    val
}

fn push_binary(tape: &mut Vec<UtmSym>, value: usize, bits: usize) {
    for i in (0..bits).rev() {
        if (value >> i) & 1 == 1 {
            tape.push(UtmSym::One);
        } else {
            tape.push(UtmSym::Zero);
        }
    }
}

fn push_binary_u64(tape: &mut Vec<UtmSym>, value: u64, bits: usize) {
    for i in (0..bits).rev() {
        if (value >> i) & 1 == 1 {
            tape.push(UtmSym::One);
        } else {
            tape.push(UtmSym::Zero);
        }
    }
}

/// Encoding parameters.
pub struct EncodingParams {
    pub state_bits: usize,
    pub symbol_bits: usize,
}

pub fn encoding_params(num_states: usize, num_symbols: usize) -> EncodingParams {
    EncodingParams {
        state_bits: bits_needed(num_states),
        symbol_bits: bits_needed(num_symbols),
    }
}

fn bits_needed(n: usize) -> usize {
    if n <= 1 { 1 } else { (usize::BITS - (n - 1).leading_zeros()) as usize }
}

fn build_codec_tables<S, A>(
    tm: &TuringMachine<S, A>,
) -> (Vec<S>, Vec<A>, EncodingParams)
where
    S: Eq + Hash + Clone + Debug,
    A: Eq + Hash + Clone + Debug,
{
    let mut states: Vec<S> = Vec::new();
    let mut symbols: Vec<A> = Vec::new();

    fn add_unique<T: Eq + Clone>(vec: &mut Vec<T>, item: &T) {
        if !vec.contains(item) {
            vec.push(item.clone());
        }
    }

    // Reject at index 0 (all-zeros)
    add_unique(&mut states, &tm.reject);
    add_unique(&mut states, &tm.initial);
    for ((s, a), (s2, a2, _)) in &tm.transitions {
        add_unique(&mut states, s);
        add_unique(&mut states, s2);
        add_unique(&mut symbols, a);
        add_unique(&mut symbols, a2);
    }
    add_unique(&mut symbols, &tm.blank);
    add_unique(&mut states, &tm.accept);

    let reject_idx = states.iter().position(|s| s == &tm.reject).unwrap();
    states.swap(0, reject_idx);
    let accept_idx = states.iter().position(|s| s == &tm.accept).unwrap();
    let last = states.len() - 1;
    states.swap(accept_idx, last);

    let params = encoding_params(states.len(), symbols.len());
    (states, symbols, params)
}

/// Decode UTM output tape back to simulated TM results.
pub fn decode<S, A>(
    tm: &TuringMachine<S, A>,
    utm_tape: &[UtmSym],
    accepted: bool,
) -> RunResult<A>
where
    S: Eq + Hash + Clone + Debug,
    A: Eq + Hash + Clone + Debug,
{
    let (_states, symbols, _params) = build_codec_tables(tm);

    let (state_bits, symbol_bits, _, _, _header_end) = parse_header(utm_tape);

    // Find #
    let hash_pos = utm_tape.iter().position(|s| matches!(s, UtmSym::Hash))
        .expect("no # in UTM output");

    let data = &utm_tape[hash_pos + 1..];
    let cells = parse_data_cells(data, state_bits, symbol_bits);

    let result_tape: Vec<A> = cells.iter()
        .map(|&(_, sym_val)| symbols[sym_val as usize].clone())
        .collect();

    RunResult {
        outcome: if accepted { Outcome::Accept } else { Outcome::Reject },
        tape: result_tape,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use crate::tm;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum FlipState { Flip, Accept, Reject }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum FlipSym { Zero, One, Blank }

    fn flip_tm() -> TuringMachine<FlipState, FlipSym> {
        let mut transitions = HashMap::new();
        transitions.insert(
            (FlipState::Flip, FlipSym::Zero),
            (FlipState::Flip, FlipSym::One, Dir::Right),
        );
        transitions.insert(
            (FlipState::Flip, FlipSym::One),
            (FlipState::Flip, FlipSym::Zero, Dir::Right),
        );
        transitions.insert(
            (FlipState::Flip, FlipSym::Blank),
            (FlipState::Accept, FlipSym::Blank, Dir::Left),
        );
        TuringMachine {
            initial: FlipState::Flip,
            accept: FlipState::Accept,
            reject: FlipState::Reject,
            blank: FlipSym::Blank,
            transitions,
        }
    }

    #[test]
    fn test_encode_produces_valid_structure() {
        let toy = flip_tm();
        let input = vec![FlipSym::Zero, FlipSym::One];
        let encoded = encode(&toy, &input);

        // Should contain exactly one #
        let hash_count = encoded.iter().filter(|s| matches!(s, UtmSym::Hash)).count();
        assert_eq!(hash_count, 1);

        // Brackets should be balanced
        let lb = encoded.iter().filter(|s| matches!(s, UtmSym::LBracket)).count();
        let rb = encoded.iter().filter(|s| matches!(s, UtmSym::RBracket)).count();
        assert_eq!(lb, rb);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let toy = flip_tm();
        let input = vec![FlipSym::Zero, FlipSym::One];
        let encoded = encode(&toy, &input);
        let decoded = decode(&toy, &encoded, false);

        assert_eq!(decoded.tape, vec![FlipSym::Zero, FlipSym::One]);
    }

    #[test]
    fn test_utm_simulates_flip_tm() {
        let toy = flip_tm();
        let input = vec![FlipSym::Zero, FlipSym::One, FlipSym::Zero];

        // Direct execution
        let direct = tm::run(&toy, &input, 1000).expect("should halt");
        assert_eq!(direct.outcome, Outcome::Accept);

        // UTM execution
        let encoded = encode(&toy, &input);
        let (utm_tape, accepted) = run_utm_on(&encoded, 100_000).expect("UTM should halt");
        let decoded = decode(&toy, &utm_tape, accepted);

        assert_eq!(decoded.outcome, direct.outcome);

        // Compare tape contents (strip trailing blanks)
        let strip_blanks = |v: &[FlipSym]| -> Vec<FlipSym> {
            let mut v = v.to_vec();
            while v.last() == Some(&FlipSym::Blank) { v.pop(); }
            v
        };

        assert_eq!(
            strip_blanks(&direct.tape),
            strip_blanks(&decoded.tape),
            "UTM and direct execution should produce same tape"
        );
    }

    #[test]
    fn test_utm_simulates_flip_empty() {
        let toy = flip_tm();
        let input: Vec<FlipSym> = vec![];

        let direct = tm::run(&toy, &input, 1000).expect("should halt");
        assert_eq!(direct.outcome, Outcome::Accept);

        let encoded = encode(&toy, &input);
        let (utm_tape, accepted) = run_utm_on(&encoded, 100_000).expect("UTM should halt");
        let decoded = decode(&toy, &utm_tape, accepted);

        assert_eq!(decoded.outcome, Outcome::Accept);
    }

    /// A TM that accepts "01" and rejects everything else.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum SimpleState { S0, S1, Accept, Reject }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum SimpleSym { Zero, One, Blank }

    fn simple_accept_01() -> TuringMachine<SimpleState, SimpleSym> {
        use SimpleState::*;
        use SimpleSym::*;
        let mut t = HashMap::new();
        // S0: expect 0
        t.insert((S0, Zero), (S1, Zero, Dir::Right));
        t.insert((S0, One), (Reject, One, Dir::Right));
        t.insert((S0, Blank), (Reject, Blank, Dir::Right));
        // S1: expect 1
        t.insert((S1, One), (Accept, One, Dir::Right));  // accept after seeing 01... but need to check nothing follows? let's keep it simple
        t.insert((S1, Zero), (Reject, Zero, Dir::Right));
        t.insert((S1, Blank), (Reject, Blank, Dir::Right));

        TuringMachine {
            initial: S0,
            accept: Accept,
            reject: Reject,
            blank: Blank,
            transitions: t,
        }
    }

    #[test]
    fn test_utm_simulates_accept_01() {
        let toy = simple_accept_01();

        // Should accept "01"
        let input = vec![SimpleSym::Zero, SimpleSym::One];
        let direct = tm::run(&toy, &input, 1000).unwrap();
        assert_eq!(direct.outcome, Outcome::Accept);

        let encoded = encode(&toy, &input);
        let (utm_tape, accepted) = run_utm_on(&encoded, 100_000).unwrap();
        let decoded = decode(&toy, &utm_tape, accepted);
        assert_eq!(decoded.outcome, Outcome::Accept);

        // Should reject "10"
        let input2 = vec![SimpleSym::One, SimpleSym::Zero];
        let direct2 = tm::run(&toy, &input2, 1000).unwrap();
        assert_eq!(direct2.outcome, Outcome::Reject);

        let encoded2 = encode(&toy, &input2);
        let (utm_tape2, accepted2) = run_utm_on(&encoded2, 100_000).unwrap();
        let decoded2 = decode(&toy, &utm_tape2, accepted2);
        assert_eq!(decoded2.outcome, Outcome::Reject);
    }
}
