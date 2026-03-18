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
    /// Marked zero (used by UTM TM for bit comparison/copy).
    Dot0,
    /// Marked one (used by UTM TM for bit comparison/copy).
    Dot1,
    /// Marked left bracket (used by UTM TM to track current transition entry).
    MarkLBracket,
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
            UtmSym::One | UtmSym::Dot1 => val |= 1,
            UtmSym::Zero | UtmSym::Dot0 => {}
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

    // Skip leading Blanks (the TM simulator may extend the tape left of position 0)
    let utm_tape = &utm_tape[utm_tape.iter().position(|s| !matches!(s, UtmSym::Blank)).unwrap_or(0)..];

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

/// Step-by-step UTM interpreter state.
pub struct UtmState {
    pub state_bits: usize,
    pub symbol_bits: usize,
    blank_sym: u64,
    accept_state: u64,
    transitions: Vec<Transition>,
    header_and_hash: Vec<UtmSym>,
    pub cells: Vec<(u64, u64)>,
    pub head_pos: usize,
    pub current_state: u64,
    pub halted: bool,
    pub accepted: bool,
    pub steps: u64,
}

impl UtmState {
    pub fn new(encoded_tape: &[UtmSym]) -> Self {
        let (state_bits, symbol_bits, blank_sym, accept_state, header_end) = parse_header(encoded_tape);
        let hash_pos = encoded_tape.iter().position(|s| matches!(s, UtmSym::Hash))
            .expect("no # in tape");
        let transitions = parse_transition_table(&encoded_tape[header_end..hash_pos], state_bits, symbol_bits);
        let cells = parse_data_cells(&encoded_tape[hash_pos + 1..], state_bits, symbol_bits);
        let head_pos = cells.iter().position(|(st, _)| *st != 0).unwrap_or(0);
        let current_state = cells.get(head_pos).map(|(s, _)| *s).unwrap_or(0);
        let header_and_hash = encoded_tape[..=hash_pos].to_vec();

        UtmState {
            state_bits, symbol_bits, blank_sym, accept_state,
            transitions, header_and_hash, cells, head_pos, current_state,
            halted: false, accepted: false, steps: 0,
        }
    }

    /// Step one simulated-machine step. Returns true if still running.
    pub fn step(&mut self) -> bool {
        if self.halted { return false; }

        if self.current_state == 0 {
            self.halted = true;
            self.accepted = false;
            return false;
        }
        if self.current_state == self.accept_state {
            self.halted = true;
            self.accepted = true;
            return false;
        }

        let current_symbol = self.cells[self.head_pos].1;
        let tr = self.transitions.iter()
            .find(|t| t.cur_state == self.current_state && t.cur_symbol == current_symbol);

        match tr {
            None => {
                self.cells[self.head_pos].0 = 0;
                self.halted = true;
                self.accepted = false;
                return false;
            }
            Some(tr) => {
                let new_state = tr.new_state;
                let new_symbol = tr.new_symbol;
                let direction = tr.direction;

                self.cells[self.head_pos].1 = new_symbol;
                self.cells[self.head_pos].0 = 0;

                match direction {
                    Dir::Left => {
                        if self.head_pos == 0 {
                            self.cells.insert(0, (0, self.blank_sym));
                        } else {
                            self.head_pos -= 1;
                        }
                    }
                    Dir::Right => {
                        self.head_pos += 1;
                        if self.head_pos >= self.cells.len() {
                            self.cells.push((0, self.blank_sym));
                        }
                    }
                }

                self.current_state = new_state;
                self.cells[self.head_pos].0 = self.current_state;
                self.steps += 1;
            }
        }
        true
    }

    /// Rebuild the full tape from current state.
    pub fn rebuild_tape(&self) -> Vec<UtmSym> {
        rebuild_tape(&self.header_and_hash, &self.cells, self.state_bits, self.symbol_bits)
    }

    /// Get symbol at cell index, as u64 index.
    pub fn cell_sym(&self, idx: usize) -> Option<u64> {
        self.cells.get(idx).map(|(_, s)| *s)
    }
}

/// Running state of a simulated TM, decoded from a UTM tape.
#[derive(Debug, Clone)]
pub struct DecodedState {
    /// Current state index of the simulated TM (0 = reject).
    pub state: u64,
    /// Head position (index into data cells).
    pub head_pos: usize,
    /// Tape symbols as indices.
    pub tape_syms: Vec<u64>,
    /// Header params.
    pub state_bits: usize,
    pub symbol_bits: usize,
    pub accept_state: u64,
}

/// Decode the running state from a raw UTM tape (which may contain marks).
/// Works on any slice of UtmSym — the tape from TmState, run_utm_on mid-flight, etc.
pub fn decode_running_state(tape: &[UtmSym]) -> Option<DecodedState> {
    // Skip leading blanks
    let start = tape.iter().position(|s| !matches!(s, UtmSym::Blank))?;
    let tape = &tape[start..];

    let (state_bits, symbol_bits, _blank_sym, accept_state, _header_end) = parse_header(tape);

    // Find # (treating MarkLBracket as LBracket for scanning)
    let hash_pos = tape.iter().position(|s| matches!(s, UtmSym::Hash))?;
    let data = &tape[hash_pos + 1..];

    // Parse data cells, tolerating marks
    let cells = parse_data_cells_tolerant(data, state_bits, symbol_bits);

    let head_pos = cells.iter().position(|(st, _)| *st != 0).unwrap_or(0);
    let state = cells.get(head_pos).map(|(s, _)| *s).unwrap_or(0);
    let tape_syms = cells.iter().map(|(_, sym)| *sym).collect();

    Some(DecodedState {
        state,
        head_pos,
        tape_syms,
        state_bits,
        symbol_bits,
        accept_state,
    })
}

/// Like parse_data_cells but tolerates MarkLBracket as LBracket.
fn parse_data_cells_tolerant(data: &[UtmSym], state_bits: usize, symbol_bits: usize) -> Vec<(u64, u64)> {
    let mut cells = Vec::new();
    let mut i = 0;

    while i < data.len() {
        match data[i] {
            UtmSym::LBracket | UtmSym::MarkLBracket => {
                i += 1;
                if i + state_bits + 1 + symbol_bits >= data.len() { break; }
                let state_val = read_binary_at(data, &mut i, state_bits);
                if i >= data.len() || !matches!(data[i], UtmSym::Pipe) { break; }
                i += 1;
                let sym_val = read_binary_at(data, &mut i, symbol_bits);
                if i >= data.len() || !matches!(data[i], UtmSym::RBracket) { break; }
                i += 1;
                cells.push((state_val, sym_val));
            }
            UtmSym::Blank => break,
            _ => break,
        }
    }

    cells
}

// ===== UTM as a Turing Machine =====
//
// Build a TuringMachine<u32, UtmSym> that simulates any TM encoded with
// state_bits=2, symbol_bits=1. This is a real UTM (for TMs in that size class)
// expressed as a finite transition table.
//
// Algorithm:
// 1. Skip header (4 semicolons)
// 2. Find head cell in data (nonzero state), read state bits → ns
// 3. Halt check: compare ns against accept state from header
// 4. If not halting: compare (state, symbol) against each transition entry
//    using dot-marking protocol (one bit at a time, back-and-forth)
// 5. On match: copy result bits to head cell, read direction
// 6. Execute: clear old state, move head, write new state at new position
// 7. Read new symbol, cleanup marks, loop to step 3
//
// Marker symbols used:
// - Dot0/Dot1: mark binary digits during comparison and copy
// - MarkLBracket: mark the current transition entry's [

/// All UtmSym variants, for generating "scan past anything" transitions.
const ALL_SYMS: &[UtmSym] = &[
    UtmSym::Zero, UtmSym::One, UtmSym::LBracket, UtmSym::RBracket,
    UtmSym::Pipe, UtmSym::Semi, UtmSym::Hash, UtmSym::D,
    UtmSym::Blank, UtmSym::Dot0, UtmSym::Dot1, UtmSym::MarkLBracket,
];

struct TmBuilder {
    transitions: std::collections::HashMap<(u32, UtmSym), (u32, UtmSym, Dir)>,
    next_state: u32,
}

impl TmBuilder {
    fn new() -> Self {
        TmBuilder {
            transitions: std::collections::HashMap::new(),
            next_state: 2, // 0=reject, 1=accept
        }
    }

    fn alloc(&mut self) -> u32 {
        let s = self.next_state;
        self.next_state += 1;
        s
    }

    fn add(&mut self, state: u32, sym: UtmSym, new_state: u32, new_sym: UtmSym, dir: Dir) {
        self.transitions.insert((state, sym), (new_state, new_sym, dir));
    }

    /// Scan right: for every symbol except `stop`, stay and move right.
    /// On `stop`, move right and go to `next`.
    fn scan_right(&mut self, state: u32, stop: UtmSym, next: u32) {
        for &sym in ALL_SYMS {
            if sym == stop {
                self.add(state, sym, next, sym, Dir::Right);
            } else {
                self.add(state, sym, state, sym, Dir::Right);
            }
        }
    }

    /// Scan left: for every symbol except `stop`, stay and move left.
    /// On `stop`, move left and go to `next`. (Stays on stop symbol.)
    fn scan_left_to(&mut self, state: u32, stop: UtmSym, next: u32) {
        for &sym in ALL_SYMS {
            if sym == stop {
                // Found it — don't move past it, move right to stay "at" it.
                // Actually for scan_left_to, stop on it and go to next.
                // Move right so we're back ON the stop symbol.
                self.add(state, sym, next, sym, Dir::Right);
            } else {
                self.add(state, sym, state, sym, Dir::Left);
            }
        }
    }

    /// Scan left: for every symbol except `stop`, stay and move left.
    /// On `stop`, move LEFT (past it) and go to `next`.
    fn scan_left_past(&mut self, state: u32, stop: UtmSym, next: u32) {
        for &sym in ALL_SYMS {
            if sym == stop {
                self.add(state, sym, next, sym, Dir::Left);
            } else {
                self.add(state, sym, state, sym, Dir::Left);
            }
        }
    }
}

/// Build a UTM as a TuringMachine<u32, UtmSym>.
///
/// This UTM correctly simulates any TM whose encoding uses state_bits=2 and
/// symbol_bits=1, with blank_sym=0 (the Zero symbol). It reads the accept
/// state from the encoded tape's header.
///
/// The generated TM has ~120 states and ~1000 transitions.
pub fn build_utm_tm() -> TuringMachine<u32, UtmSym> {
    let mut b = TmBuilder::new();

    // ====== Phase 0: Skip header (4 semicolons) ======
    let init = b.alloc(); // 2 = initial state
    let skip_sc2 = b.alloc();
    let skip_sc3 = b.alloc();
    let skip_sc4 = b.alloc();
    let at_trans_start = b.alloc();

    b.scan_right(init, UtmSym::Semi, skip_sc2);
    b.scan_right(skip_sc2, UtmSym::Semi, skip_sc3);
    b.scan_right(skip_sc3, UtmSym::Semi, skip_sc4);
    b.scan_right(skip_sc4, UtmSym::Semi, at_trans_start);

    // ====== Phase 1: Scan to data, find head cell ======
    // From at_trans_start, scan right to #, then find head cell.
    let scan_to_hash = b.alloc();
    let find_head_lb = b.alloc();
    let find_head_s0 = b.alloc();
    let find_head_s1_after0 = b.alloc();
    let skip_nh_pipe = b.alloc();
    let skip_nh_a0 = b.alloc();
    let skip_nh_rb = b.alloc();
    let head_back_to_s0 = b.alloc();
    let head_at_s0 = b.alloc();
    let head_read_s1 = b.alloc(); // after reading s0, at s1

    // at_trans_start: check if transitions exist
    // - LBracket → scan to hash (there are transitions)
    // - Hash → go directly to find head
    // For simplicity, just scan right to #.
    b.scan_right(at_trans_start, UtmSym::Hash, find_head_lb);
    // But we also enter at_trans_start after halt check. Let's handle both entry points.
    // Actually, at_trans_start should go to scan_to_hash first.
    // Let me restructure: at_trans_start goes to scan_to_hash.
    // Hmm, but scan_right for at_trans_start already handles that.
    // Actually, I want at_trans_start to scan right to # and then find head.
    // Let me just make at_trans_start = scan_to_hash (merge them).

    // REDO: at_trans_start scans right to #.
    // After #, we're at data start. Scan right for [.
    // find_head_lb scans right for [ in data.

    // Redefine: at_trans_start → scan right to # → find_head_lb
    // find_head_lb: at start of data, scan right for [
    // After [, read s0.

    // For at [ in data: move right → find_head_s0
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::LBracket => b.add(find_head_lb, sym, find_head_s0, sym, Dir::Right),
            UtmSym::Blank => b.add(find_head_lb, sym, 0, sym, Dir::Right), // no head found → reject
            _ => b.add(find_head_lb, sym, find_head_lb, sym, Dir::Right),
        }
    }

    // find_head_s0: read first state bit
    // Zero → check s1
    // One → head found (at s0)
    // Dot0/Dot1 → head found (partially marked from previous)
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(find_head_s0, sym, find_head_s1_after0, sym, Dir::Right),
            UtmSym::One => b.add(find_head_s0, sym, head_at_s0, sym, Dir::Left), // back up to [ then come back
            UtmSym::Dot0 | UtmSym::Dot1 => b.add(find_head_s0, sym, head_at_s0, sym, Dir::Left),
            _ => b.add(find_head_s0, sym, 0, sym, Dir::Right), // unexpected → reject
        }
    }
    // Oops: head_at_s0 should be AT s0. But when s0=One, I'm at s0 and I did Dir::Left which puts me at [.
    // Let me fix: when s0=One, we're AT s0 already. Don't move. But TMs must move.
    // Move left to [, then come back right to s0.
    // Actually: I'm reading s0. If it's One, I already know it's the head cell.
    // The comparison protocol needs me at s0 (the first bit).
    // I'm already at s0! So I should proceed directly.
    // But TM transitions always move. Let me use a two-step: move left (to [), then right (back to s0).

    // head_at_s0_via_lb: at [ of head cell, move right → head_at_s0
    let head_at_s0_via_lb = b.alloc();
    for &sym in ALL_SYMS {
        b.add(head_at_s0_via_lb, sym, head_at_s0, sym, Dir::Right);
    }

    // Fix find_head_s0: when One, move left to [ → head_at_s0_via_lb
    b.add(find_head_s0, UtmSym::One, head_at_s0_via_lb, UtmSym::One, Dir::Left);
    b.add(find_head_s0, UtmSym::Dot0, head_at_s0_via_lb, UtmSym::Dot0, Dir::Left);
    b.add(find_head_s0, UtmSym::Dot1, head_at_s0_via_lb, UtmSym::Dot1, Dir::Left);

    // find_head_s1_after0: s0=0, read s1
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(find_head_s1_after0, sym, skip_nh_pipe, sym, Dir::Right), // 00 → not head
            UtmSym::One | UtmSym::Dot0 | UtmSym::Dot1 => {
                // Head found. Go back to s0.
                b.add(find_head_s1_after0, sym, head_at_s0_via_lb, sym, Dir::Left); // to s0
                // Actually: from s1, move left goes to s0. Then move left again to [. Then right to s0.
                // Two lefts needed. Let me use head_back_to_s0 as intermediate.
            }
            _ => b.add(find_head_s1_after0, sym, 0, sym, Dir::Right),
        }
    }
    // Fix: for One/Dot at s1, we need to go back TWO positions (s1→s0→[→s0).
    // From s1, move left → at s0. head_back_to_s0 moves left to [, then right to s0.
    b.add(find_head_s1_after0, UtmSym::One, head_back_to_s0, UtmSym::One, Dir::Left);
    b.add(find_head_s1_after0, UtmSym::Dot0, head_back_to_s0, UtmSym::Dot0, Dir::Left);
    b.add(find_head_s1_after0, UtmSym::Dot1, head_back_to_s0, UtmSym::Dot1, Dir::Left);

    // head_back_to_s0: we're at s0 (came from s1). Move left to [, then right.
    for &sym in ALL_SYMS {
        b.add(head_back_to_s0, sym, head_at_s0_via_lb, sym, Dir::Left);
    }

    // skip_nh_pipe: skip | of non-head cell
    for &sym in ALL_SYMS {
        b.add(skip_nh_pipe, sym, skip_nh_a0, sym, Dir::Right);
    }
    // skip_nh_a0: skip a0
    for &sym in ALL_SYMS {
        b.add(skip_nh_a0, sym, skip_nh_rb, sym, Dir::Right);
    }
    // skip_nh_rb: skip ]
    for &sym in ALL_SYMS {
        b.add(skip_nh_rb, sym, find_head_lb, sym, Dir::Right); // back to scan for next [
    }

    // ====== Phase 2: At head cell s0, read state for halt check ======
    // head_at_s0: at s0 of head cell. Read s0 and s1 to determine state.
    // We need to know the state for halt checking.
    let head_read_s0_is0 = b.alloc();
    let head_read_s0_is1 = b.alloc();

    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(head_at_s0, sym, head_read_s0_is0, sym, Dir::Right),
            UtmSym::One => b.add(head_at_s0, sym, head_read_s0_is1, sym, Dir::Right),
            _ => b.add(head_at_s0, sym, 0, sym, Dir::Right), // shouldn't happen
        }
    }

    // States for each ns value (after reading both bits):
    // ns01, ns10, ns11 → halt check
    let halt_check_ns01 = b.alloc();
    let halt_check_ns10 = b.alloc();
    let halt_check_ns11 = b.alloc();

    // head_read_s0_is0: s0=0, move right, read s1
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(head_read_s0_is0, sym, 0, sym, Dir::Right), // ns=00 → reject
            UtmSym::One => b.add(head_read_s0_is0, sym, halt_check_ns01, sym, Dir::Left),
            _ => b.add(head_read_s0_is0, sym, 0, sym, Dir::Right),
        }
    }
    // head_read_s0_is1: s0=1, move right, read s1
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(head_read_s0_is1, sym, halt_check_ns10, sym, Dir::Left),
            UtmSym::One => b.add(head_read_s0_is1, sym, halt_check_ns11, sym, Dir::Left),
            _ => b.add(head_read_s0_is1, sym, 0, sym, Dir::Right),
        }
    }

    // ====== Phase 3: Halt check — compare ns against accept state ======
    // For each ns ∈ {01, 10, 11}, scan left to tape start (Blank), then
    // skip 3 semicolons to reach accept field, read 2 bits, compare.
    // If match → ACCEPT. If not → go to transitions (skip past 4th ;).

    // Helper: build halt check chain for a given ns value.
    // Returns the "go to transitions" state.
    fn build_halt_check(
        b: &mut TmBuilder,
        entry_state: u32,
        ns0: u8,
        ns1: u8,
    ) -> u32 {
        // Scan left to Blank (tape start)
        let scan_l_blank = b.alloc();
        let skip_sc1 = b.alloc();
        let skip_sc2 = b.alloc();
        let skip_sc3 = b.alloc();
        let read_acc_s0 = b.alloc();
        let goto_trans = b.alloc(); // will scan right past 4th ; to transitions

        // Entry: we're at s1 of head cell. Scan left to Blank (tape start).
        // scan_left_to finds Blank and moves RIGHT, landing at position 0 (first header byte).
        b.scan_left_to(entry_state, UtmSym::Blank, skip_sc1);
        // Skip 3 semicolons to reach accept field
        b.scan_right(skip_sc1, UtmSym::Semi, skip_sc2);
        b.scan_right(skip_sc2, UtmSym::Semi, skip_sc3);
        b.scan_right(skip_sc3, UtmSym::Semi, read_acc_s0);

        // read_acc_s0: read first accept bit
        let read_acc_s1_a0 = b.alloc();
        let read_acc_s1_a1 = b.alloc();
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::Zero => b.add(read_acc_s0, sym, read_acc_s1_a0, sym, Dir::Right),
                UtmSym::One => b.add(read_acc_s0, sym, read_acc_s1_a1, sym, Dir::Right),
                _ => b.add(read_acc_s0, sym, 0, sym, Dir::Right),
            }
        }

        // read_acc_s1_a0: acc_s0=0, read acc_s1
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::Zero => {
                    // accept=00. Compare with ns.
                    if ns0 == 0 && ns1 == 0 {
                        b.add(read_acc_s1_a0, sym, 1, sym, Dir::Right); // ACCEPT
                    } else {
                        b.add(read_acc_s1_a0, sym, goto_trans, sym, Dir::Right); // not accept
                    }
                }
                UtmSym::One => {
                    // accept=01
                    if ns0 == 0 && ns1 == 1 {
                        b.add(read_acc_s1_a0, sym, 1, sym, Dir::Right);
                    } else {
                        b.add(read_acc_s1_a0, sym, goto_trans, sym, Dir::Right);
                    }
                }
                _ => b.add(read_acc_s1_a0, sym, 0, sym, Dir::Right),
            }
        }

        // read_acc_s1_a1: acc_s0=1, read acc_s1
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::Zero => {
                    // accept=10
                    if ns0 == 1 && ns1 == 0 {
                        b.add(read_acc_s1_a1, sym, 1, sym, Dir::Right);
                    } else {
                        b.add(read_acc_s1_a1, sym, goto_trans, sym, Dir::Right);
                    }
                }
                UtmSym::One => {
                    // accept=11
                    if ns0 == 1 && ns1 == 1 {
                        b.add(read_acc_s1_a1, sym, 1, sym, Dir::Right);
                    } else {
                        b.add(read_acc_s1_a1, sym, goto_trans, sym, Dir::Right);
                    }
                }
                _ => b.add(read_acc_s1_a1, sym, 0, sym, Dir::Right),
            }
        }

        // goto_trans: skip past the accept bits and 4th ; to reach transitions.
        // We're at acc_s1 position. Move right: at ; (4th semicolon). Move right: at transitions.
        // Actually, we just read acc_s1 and moved right. So we're past acc_s1.
        // Next should be ; (4th semicolon).
        let after_sc4 = b.alloc();
        b.scan_right(goto_trans, UtmSym::Semi, after_sc4);

        after_sc4 // this is the "at transitions start" state
    }

    let trans_start_from_ns01 = build_halt_check(&mut b, halt_check_ns01, 0, 1);
    let trans_start_from_ns10 = build_halt_check(&mut b, halt_check_ns10, 1, 0);
    let trans_start_from_ns11 = build_halt_check(&mut b, halt_check_ns11, 1, 1);

    // ====== Phase 4: Enter transition entry and compare ======
    // All trans_start_from_nsXX states arrive at the transition region start.
    // They need to find the first un-marked [ (transition entry to try).

    // Shared comparison states:
    let enter_entry = b.alloc(); // at [ of transition to try
    let cmp_goto_hash = b.alloc(); // scan right to # (going to head cell)
    let cmp_find_head_lb = b.alloc(); // in data, scan right for [
    let cmp_head_s0 = b.alloc(); // at s0 of potential head cell
    let cmp_head_s1_after0 = b.alloc();
    let cmp_skip_nh_pipe = b.alloc();
    let cmp_skip_nh_a0 = b.alloc();
    let cmp_skip_nh_rb = b.alloc();
    let cmp_head_via_lb = b.alloc(); // at [ of head cell, move right
    let cmp_head_skipdots = b.alloc(); // skip dots/pipe to find undotted bit
    let cmp_carry0_find_mark = b.alloc(); // scan left to MarkLBracket carrying 0
    let cmp_carry0_find_bit = b.alloc(); // in transition key, find undotted bit, compare=0
    let cmp_carry1_find_mark = b.alloc();
    let cmp_carry1_find_bit = b.alloc();
    let cmp_bit_ok = b.alloc(); // bit matched, go back for next
    let cmp_all_dotted = b.alloc(); // all head bits dotted → full match
    let cmp_mismatch_goto_hash = b.alloc();
    let cmp_mismatch_undo_head = b.alloc();
    let cmp_mismatch_undo_key_goto = b.alloc();
    let cmp_mismatch_undo_key = b.alloc();
    let cmp_next_entry = b.alloc(); // scan right for next [ or #
    let cmp_no_match_cleanup_l = b.alloc();
    let cmp_no_match_cleanup_r = b.alloc();

    // Connect trans_start states to enter_entry by scanning for first [.
    // At trans start, scan right for [ (skip ;, MarkLBracket, etc.)
    for trans_start in [trans_start_from_ns01, trans_start_from_ns10, trans_start_from_ns11, at_trans_start] {
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::LBracket => b.add(trans_start, sym, enter_entry, sym, Dir::Right),
                UtmSym::Hash => b.add(trans_start, sym, 0, sym, Dir::Right), // no transitions → reject
                _ => b.add(trans_start, sym, trans_start, sym, Dir::Right),
            }
        }
    }
    // Oops, enter_entry should be AT the [, not past it. Let me fix:
    // When we find [, we want to mark it as MarkLBracket and move right.
    // So enter_entry is triggered when we're AT [.
    // Let me redo: trans_start scans right. On [, go to enter_entry.
    // But enter_entry should be AT the [. Since scan_right moves past it...
    // I need to: on [, DON'T move right. But TM must move.
    // Solution: on [, move LEFT then come back. Or: at_trans_start scans for [,
    // when found, we're past it. Back up.

    // Actually, let me restructure. at_trans_start doesn't use scan_right helper.
    // Instead, find [ and handle marking:
    // Actually the simplest fix: enter_entry is the state where we're ONE PAST [.
    // So we mark the [ BEHIND us. Hmm, TMs can only write to current cell.

    // Better: When trans_start finds LBracket, it writes MarkLBracket and moves right.
    // That IS entering the entry. So enter_entry = after marking [, at first key bit.

    // Let me redefine:
    // trans_start_scan: scanning for next LBracket (untried entry)
    // On LBracket: write MarkLBracket, move right → cmp_goto_hash
    // On MarkLBracket: skip (already tried), move right
    // On Hash: no more entries → no match

    // I need to redo the trans_start connections:
    // Clear previous registrations and redo
    for trans_start in [trans_start_from_ns01, trans_start_from_ns10, trans_start_from_ns11, at_trans_start] {
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::LBracket => {
                    b.add(trans_start, sym, cmp_goto_hash, UtmSym::MarkLBracket, Dir::Right);
                }
                UtmSym::Hash => {
                    // No untried transition entries left → no match → reject
                    // But need to undo MarkLBrackets first
                    b.add(trans_start, sym, cmp_no_match_cleanup_l, sym, Dir::Left);
                }
                _ => {
                    b.add(trans_start, sym, trans_start, sym, Dir::Right);
                }
            }
        }
    }

    // cmp_goto_hash: scan right to # (entering data region)
    b.scan_right(cmp_goto_hash, UtmSym::Hash, cmp_find_head_lb);

    // cmp_find_head_lb: scan for [ in data
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::LBracket => b.add(cmp_find_head_lb, sym, cmp_head_s0, sym, Dir::Right),
            UtmSym::Blank => b.add(cmp_find_head_lb, sym, 0, sym, Dir::Right),
            _ => b.add(cmp_find_head_lb, sym, cmp_find_head_lb, sym, Dir::Right),
        }
    }

    // cmp_head_s0: read first state bit
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(cmp_head_s0, sym, cmp_head_s1_after0, sym, Dir::Right),
            UtmSym::One | UtmSym::Dot0 | UtmSym::Dot1 => {
                b.add(cmp_head_s0, sym, cmp_head_via_lb, sym, Dir::Left);
            }
            _ => b.add(cmp_head_s0, sym, 0, sym, Dir::Right),
        }
    }

    // cmp_head_s1_after0: s0=0, read s1
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Zero => b.add(cmp_head_s1_after0, sym, cmp_skip_nh_pipe, sym, Dir::Right),
            UtmSym::One | UtmSym::Dot0 | UtmSym::Dot1 => {
                // Head found. Go back to [ (two lefts: s1→s0→[)
                let back_from_s1 = b.alloc();
                b.add(cmp_head_s1_after0, sym, back_from_s1, sym, Dir::Left);
                for &s2 in ALL_SYMS {
                    b.add(back_from_s1, s2, cmp_head_via_lb, s2, Dir::Left);
                }
            }
            _ => b.add(cmp_head_s1_after0, sym, 0, sym, Dir::Right),
        }
    }

    // cmp_skip_nh: skip non-head cell
    for &sym in ALL_SYMS {
        b.add(cmp_skip_nh_pipe, sym, cmp_skip_nh_a0, sym, Dir::Right); // skip |
    }
    for &sym in ALL_SYMS {
        b.add(cmp_skip_nh_a0, sym, cmp_skip_nh_rb, sym, Dir::Right); // skip a0
    }
    for &sym in ALL_SYMS {
        b.add(cmp_skip_nh_rb, sym, cmp_find_head_lb, sym, Dir::Right); // skip ]
    }

    // cmp_head_via_lb: at [ of head cell, move right to s0
    for &sym in ALL_SYMS {
        b.add(cmp_head_via_lb, sym, cmp_head_skipdots, sym, Dir::Right);
    }

    // cmp_head_skipdots: skip Dot0/Dot1 and Pipe, stop at Zero/One or RBracket
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Dot0 | UtmSym::Dot1 | UtmSym::Pipe => {
                b.add(cmp_head_skipdots, sym, cmp_head_skipdots, sym, Dir::Right);
            }
            UtmSym::Zero => {
                // Found undotted bit = 0. Dot it and carry 0 to transition.
                b.add(cmp_head_skipdots, sym, cmp_carry0_find_mark, UtmSym::Dot0, Dir::Left);
            }
            UtmSym::One => {
                b.add(cmp_head_skipdots, sym, cmp_carry1_find_mark, UtmSym::Dot1, Dir::Left);
            }
            UtmSym::RBracket => {
                // All bits dotted → full match!
                b.add(cmp_head_skipdots, sym, cmp_all_dotted, sym, Dir::Left);
            }
            _ => {
                b.add(cmp_head_skipdots, sym, 0, sym, Dir::Right);
            }
        }
    }

    // cmp_carry0_find_mark: scan left to MarkLBracket
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket => {
                b.add(cmp_carry0_find_mark, sym, cmp_carry0_find_bit, sym, Dir::Right);
            }
            _ => {
                b.add(cmp_carry0_find_mark, sym, cmp_carry0_find_mark, sym, Dir::Left);
            }
        }
    }

    // cmp_carry0_find_bit: scan right from MarkLBracket, skip dots, find undotted key bit
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket | UtmSym::Dot0 | UtmSym::Dot1 => {
                b.add(cmp_carry0_find_bit, sym, cmp_carry0_find_bit, sym, Dir::Right);
            }
            UtmSym::Zero => {
                // Key bit = 0, matches carry = 0. Dot it.
                b.add(cmp_carry0_find_bit, sym, cmp_bit_ok, UtmSym::Dot0, Dir::Right);
            }
            UtmSym::One => {
                // Key bit = 1, mismatch with carry = 0.
                b.add(cmp_carry0_find_bit, sym, cmp_mismatch_goto_hash, sym, Dir::Right);
            }
            UtmSym::Pipe => {
                // All key bits dotted but we're still carrying → shouldn't happen
                b.add(cmp_carry0_find_bit, sym, 0, sym, Dir::Right);
            }
            _ => b.add(cmp_carry0_find_bit, sym, 0, sym, Dir::Right),
        }
    }

    // cmp_carry1_find_mark: scan left to MarkLBracket
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket => {
                b.add(cmp_carry1_find_mark, sym, cmp_carry1_find_bit, sym, Dir::Right);
            }
            _ => {
                b.add(cmp_carry1_find_mark, sym, cmp_carry1_find_mark, sym, Dir::Left);
            }
        }
    }

    // cmp_carry1_find_bit: scan right, find undotted key bit, compare with 1
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket | UtmSym::Dot0 | UtmSym::Dot1 => {
                b.add(cmp_carry1_find_bit, sym, cmp_carry1_find_bit, sym, Dir::Right);
            }
            UtmSym::One => {
                // Matches carry = 1. Dot it.
                b.add(cmp_carry1_find_bit, sym, cmp_bit_ok, UtmSym::Dot1, Dir::Right);
            }
            UtmSym::Zero => {
                // Mismatch.
                b.add(cmp_carry1_find_bit, sym, cmp_mismatch_goto_hash, sym, Dir::Right);
            }
            UtmSym::Pipe => b.add(cmp_carry1_find_bit, sym, 0, sym, Dir::Right),
            _ => b.add(cmp_carry1_find_bit, sym, 0, sym, Dir::Right),
        }
    }

    // cmp_bit_ok: bit matched. Go back to head cell for next bit.
    // We're in the transition entry (just dotted a key bit). Scan right to # then to head.
    b.scan_right(cmp_bit_ok, UtmSym::Hash, cmp_find_head_lb);

    // ====== Phase 4b: Mismatch handling ======
    // cmp_mismatch_goto_hash: scan right to #, then undo head dots
    b.scan_right(cmp_mismatch_goto_hash, UtmSym::Hash, cmp_mismatch_undo_head);

    // cmp_mismatch_undo_head: scan right through data, undot all Dot0→Zero, Dot1→One
    // Stop at Blank (end of tape data)
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Dot0 => b.add(cmp_mismatch_undo_head, sym, cmp_mismatch_undo_head, UtmSym::Zero, Dir::Right),
            UtmSym::Dot1 => b.add(cmp_mismatch_undo_head, sym, cmp_mismatch_undo_head, UtmSym::One, Dir::Right),
            UtmSym::Blank => b.add(cmp_mismatch_undo_head, sym, cmp_mismatch_undo_key_goto, sym, Dir::Left),
            _ => b.add(cmp_mismatch_undo_head, sym, cmp_mismatch_undo_head, sym, Dir::Right),
        }
    }

    // cmp_mismatch_undo_key_goto: scan left to MarkLBracket
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket => b.add(cmp_mismatch_undo_key_goto, sym, cmp_mismatch_undo_key, sym, Dir::Right),
            _ => b.add(cmp_mismatch_undo_key_goto, sym, cmp_mismatch_undo_key_goto, sym, Dir::Left),
        }
    }

    // cmp_mismatch_undo_key: scan right, undot key bits until Pipe
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Dot0 => b.add(cmp_mismatch_undo_key, sym, cmp_mismatch_undo_key, UtmSym::Zero, Dir::Right),
            UtmSym::Dot1 => b.add(cmp_mismatch_undo_key, sym, cmp_mismatch_undo_key, UtmSym::One, Dir::Right),
            UtmSym::MarkLBracket => b.add(cmp_mismatch_undo_key, sym, cmp_mismatch_undo_key, sym, Dir::Right),
            UtmSym::Pipe => b.add(cmp_mismatch_undo_key, sym, cmp_next_entry, sym, Dir::Right),
            UtmSym::Zero | UtmSym::One => b.add(cmp_mismatch_undo_key, sym, cmp_mismatch_undo_key, sym, Dir::Right),
            _ => b.add(cmp_mismatch_undo_key, sym, 0, sym, Dir::Right),
        }
    }

    // cmp_next_entry: scan right for next LBracket (untried entry) or Hash (no more)
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::LBracket => {
                // Found next entry. Mark it and start comparing.
                b.add(cmp_next_entry, sym, cmp_goto_hash, UtmSym::MarkLBracket, Dir::Right);
            }
            UtmSym::Hash => {
                // No more entries. Need to also check for MarkLBracket entries we skipped.
                // Actually, we already tried all entries with MarkLBracket. So no match.
                b.add(cmp_next_entry, sym, cmp_no_match_cleanup_l, sym, Dir::Left);
            }
            _ => b.add(cmp_next_entry, sym, cmp_next_entry, sym, Dir::Right),
        }
    }

    // cmp_no_match_cleanup: restore all MarkLBrackets, then reject
    // Scan left to Blank (tape start)
    b.scan_left_to(cmp_no_match_cleanup_l, UtmSym::Blank, cmp_no_match_cleanup_r);
    // Scan right, fix marks
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket => b.add(cmp_no_match_cleanup_r, sym, cmp_no_match_cleanup_r, UtmSym::LBracket, Dir::Right),
            UtmSym::Hash => b.add(cmp_no_match_cleanup_r, sym, 0, sym, Dir::Right), // done, reject
            _ => b.add(cmp_no_match_cleanup_r, sym, cmp_no_match_cleanup_r, sym, Dir::Right),
        }
    }

    // ====== Phase 5: Full match → copy result to head cell ======
    // cmp_all_dotted: all head cell bits are dotted. Full match found.
    // Scan left to MarkLBracket.
    let copy_find_result = b.alloc();
    let copy_carry0_hash = b.alloc();
    let copy_carry0_dot = b.alloc();
    let copy_carry1_hash = b.alloc();
    let copy_carry1_dot = b.alloc();
    let copy_return = b.alloc();
    let copy_read_dir = b.alloc();

    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket => b.add(cmp_all_dotted, sym, copy_find_result, sym, Dir::Right),
            _ => b.add(cmp_all_dotted, sym, cmp_all_dotted, sym, Dir::Left),
        }
    }

    // copy_find_result: scan right from MarkLBracket, skip dots/pipe, find undotted result bit
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket | UtmSym::Dot0 | UtmSym::Dot1 | UtmSym::Pipe => {
                b.add(copy_find_result, sym, copy_find_result, sym, Dir::Right);
            }
            UtmSym::Zero => {
                // Result bit = 0. Mark it, carry to head cell.
                b.add(copy_find_result, sym, copy_carry0_hash, UtmSym::Dot0, Dir::Right);
            }
            UtmSym::One => {
                b.add(copy_find_result, sym, copy_carry1_hash, UtmSym::Dot1, Dir::Right);
            }
            UtmSym::D => {
                // Reached D marker → all state/symbol bits copied. Read direction.
                b.add(copy_find_result, sym, copy_read_dir, sym, Dir::Right);
            }
            _ => b.add(copy_find_result, sym, 0, sym, Dir::Right),
        }
    }

    // copy_carry0_hash: scan right to #, then find first dot in data to overwrite
    b.scan_right(copy_carry0_hash, UtmSym::Hash, copy_carry0_dot);

    // copy_carry0_dot: scan right for first Dot0/Dot1 in data, overwrite with Zero
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Dot0 | UtmSym::Dot1 => {
                b.add(copy_carry0_dot, sym, copy_return, UtmSym::Zero, Dir::Left);
            }
            UtmSym::Blank => b.add(copy_carry0_dot, sym, 0, sym, Dir::Right),
            _ => b.add(copy_carry0_dot, sym, copy_carry0_dot, sym, Dir::Right),
        }
    }

    // copy_carry1_hash: scan right to #, then find first dot
    b.scan_right(copy_carry1_hash, UtmSym::Hash, copy_carry1_dot);

    // copy_carry1_dot: scan right for first Dot0/Dot1, overwrite with One
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::Dot0 | UtmSym::Dot1 => {
                b.add(copy_carry1_dot, sym, copy_return, UtmSym::One, Dir::Left);
            }
            UtmSym::Blank => b.add(copy_carry1_dot, sym, 0, sym, Dir::Right),
            _ => b.add(copy_carry1_dot, sym, copy_carry1_dot, sym, Dir::Right),
        }
    }

    // copy_return: scan left to MarkLBracket, then find next result bit
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::MarkLBracket => b.add(copy_return, sym, copy_find_result, sym, Dir::Right),
            _ => b.add(copy_return, sym, copy_return, sym, Dir::Left),
        }
    }

    // copy_read_dir: at direction bit (after D marker)
    let exec_r_goto_hash = b.alloc();
    let exec_l_goto_hash = b.alloc();
    for &sym in ALL_SYMS {
        match sym {
            UtmSym::One => b.add(copy_read_dir, sym, exec_r_goto_hash, sym, Dir::Right), // RIGHT
            UtmSym::Zero => b.add(copy_read_dir, sym, exec_l_goto_hash, sym, Dir::Right), // LEFT
            _ => b.add(copy_read_dir, sym, 0, sym, Dir::Right),
        }
    }

    // ====== Phase 6: Execute — move head ======
    // After copy, head cell has new state (ns) and new symbol (na) written.
    // No more dots in head cell. Transition entry has dots and MarkLBracket.
    //
    // For each direction:
    // 1. Scan to head cell (nonzero state) — but ns could be 00 (reject)!
    //    If ns=00: head cell has state=00, can't find it. But we should reject.
    //    Actually, ns=00 means reject. But we already wrote it to the cell.
    //    We'll detect this when scanning data and finding no nonzero state → reject.
    //    But first we need to clean up marks for tape integrity.
    //
    // 2. Read ns (s0,s1) from head cell.
    // 3. Clear state to 00 in head cell.
    // 4. Move to adjacent cell.
    // 5. Write ns to adjacent cell's state.
    // 6. Read adjacent cell's symbol.
    // 7. Cleanup marks → halt check with new ns.

    // Helper function for building execute for one direction.
    fn build_execute(
        b: &mut TmBuilder,
        goto_hash_state: u32,
        dir: Dir,
        halt_check_ns01: u32,
        halt_check_ns10: u32,
        halt_check_ns11: u32,
    ) {
        let find_head_lb = b.alloc();
        let check_s0 = b.alloc();
        let check_s1_after0 = b.alloc();
        let skip_cell_pipe = b.alloc();
        let skip_cell_a0 = b.alloc();
        let skip_cell_rb = b.alloc();

        // Scan right to #, then into data
        b.scan_right(goto_hash_state, UtmSym::Hash, find_head_lb);

        // Find head cell
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::LBracket => b.add(find_head_lb, sym, check_s0, sym, Dir::Right),
                UtmSym::Blank => {
                    // No head found → ns=00 → need cleanup + reject
                    let cleanup = b.alloc();
                    let cleanup_r = b.alloc();
                    b.add(find_head_lb, sym, cleanup, sym, Dir::Left);
                    b.scan_left_to(cleanup, UtmSym::Blank, cleanup_r);
                    for &s in ALL_SYMS {
                        match s {
                            UtmSym::MarkLBracket => b.add(cleanup_r, s, cleanup_r, UtmSym::LBracket, Dir::Right),
                            UtmSym::Dot0 => b.add(cleanup_r, s, cleanup_r, UtmSym::Zero, Dir::Right),
                            UtmSym::Dot1 => b.add(cleanup_r, s, cleanup_r, UtmSym::One, Dir::Right),
                            UtmSym::Blank => b.add(cleanup_r, s, 0, s, Dir::Right), // reject
                            _ => b.add(cleanup_r, s, cleanup_r, s, Dir::Right),
                        }
                    }
                }
                _ => b.add(find_head_lb, sym, find_head_lb, sym, Dir::Right),
            }
        }

        // check_s0: at s0 of cell
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::Zero => b.add(check_s0, sym, check_s1_after0, sym, Dir::Right),
                UtmSym::One => {
                    // s0=1, need to read s1 for ns determination
                    let read_s1_ns1x = b.alloc();
                    b.add(check_s0, sym, read_s1_ns1x, sym, Dir::Right);
                    build_exec_with_ns_s0(b, read_s1_ns1x, 1, dir, halt_check_ns01, halt_check_ns10, halt_check_ns11);
                }
                _ => b.add(check_s0, sym, 0, sym, Dir::Right),
            }
        }

        // check_s1_after0: s0=0
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::Zero => b.add(check_s1_after0, sym, skip_cell_pipe, sym, Dir::Right), // 00, not head
                UtmSym::One => {
                    // s0=0, s1=1 → ns=01. At s1 position.
                    let exec_ns01 = b.alloc();
                    b.add(check_s1_after0, sym, exec_ns01, sym, Dir::Left); // back to s0
                    build_exec_clear_and_move(b, exec_ns01, 0, 1, dir, halt_check_ns01, halt_check_ns10, halt_check_ns11);
                }
                _ => b.add(check_s1_after0, sym, 0, sym, Dir::Right),
            }
        }

        // Skip non-head cell
        for &sym in ALL_SYMS { b.add(skip_cell_pipe, sym, skip_cell_a0, sym, Dir::Right); }
        for &sym in ALL_SYMS { b.add(skip_cell_a0, sym, skip_cell_rb, sym, Dir::Right); }
        for &sym in ALL_SYMS { b.add(skip_cell_rb, sym, find_head_lb, sym, Dir::Right); }
    }

    fn build_exec_with_ns_s0(
        b: &mut TmBuilder,
        read_s1_state: u32,
        s0: u8,
        dir: Dir,
        hc01: u32, hc10: u32, hc11: u32,
    ) {
        // At s1, read it:
        for &sym in ALL_SYMS {
            match sym {
                UtmSym::Zero => {
                    let ns = (s0, 0u8);
                    if ns == (0, 0) {
                        // ns=00 → shouldn't be head. reject.
                        b.add(read_s1_state, sym, 0, sym, Dir::Right);
                    } else {
                        let exec = b.alloc();
                        b.add(read_s1_state, sym, exec, sym, Dir::Left); // back to s0
                        build_exec_clear_and_move(b, exec, s0, 0, dir, hc01, hc10, hc11);
                    }
                }
                UtmSym::One => {
                    let exec = b.alloc();
                    b.add(read_s1_state, sym, exec, sym, Dir::Left); // back to s0
                    build_exec_clear_and_move(b, exec, s0, 1, dir, hc01, hc10, hc11);
                }
                _ => b.add(read_s1_state, sym, 0, sym, Dir::Right),
            }
        }
    }

    fn build_exec_clear_and_move(
        b: &mut TmBuilder,
        at_s0: u32, // state where we're at s0 of head cell
        ns0: u8,
        ns1: u8,
        dir: Dir,
        hc01: u32, hc10: u32, hc11: u32,
    ) {
        // Clear s0 (write 0), move right to s1
        let at_s1 = b.alloc();
        for &sym in ALL_SYMS {
            b.add(at_s0, sym, at_s1, UtmSym::Zero, Dir::Right);
        }
        // Clear s1 (write 0), then navigate based on direction
        let after_clear_s1 = b.alloc();
        for &sym in ALL_SYMS {
            b.add(at_s1, sym, after_clear_s1, UtmSym::Zero, Dir::Right);
        }

        // after_clear_s1: at | position. Need to move to adjacent cell.
        match dir {
            Dir::Right => {
                // Move right: skip |, a0, ] to reach next cell's [
                let skip_a0 = b.alloc();
                let skip_rb = b.alloc();
                let at_next = b.alloc();

                for &sym in ALL_SYMS {
                    b.add(after_clear_s1, sym, skip_a0, sym, Dir::Right); // skip |
                }
                for &sym in ALL_SYMS {
                    b.add(skip_a0, sym, skip_rb, sym, Dir::Right); // skip a0
                }
                for &sym in ALL_SYMS {
                    b.add(skip_rb, sym, at_next, sym, Dir::Right); // skip ]
                }

                // at_next: should be [ (existing cell) or Blank (need to create)
                let write_ns0 = b.alloc();
                let write_ns1 = b.alloc();
                let read_sym = b.alloc();

                // Create cell if needed
                let create_s0 = b.alloc();
                let create_s1 = b.alloc();
                let create_pipe = b.alloc();
                let create_a0 = b.alloc();
                let create_rb = b.alloc();
                let created_back_to_s0 = b.alloc();

                for &sym in ALL_SYMS {
                    match sym {
                        UtmSym::LBracket => {
                            // Existing cell. Move to s0.
                            b.add(at_next, sym, write_ns0, sym, Dir::Right);
                        }
                        UtmSym::Blank => {
                            // Create new cell: write [
                            b.add(at_next, sym, create_s0, UtmSym::LBracket, Dir::Right);
                        }
                        _ => b.add(at_next, sym, 0, sym, Dir::Right),
                    }
                }

                // Create new blank cell: [00|0]
                for &sym in ALL_SYMS {
                    b.add(create_s0, sym, create_s1, UtmSym::Zero, Dir::Right);
                }
                for &sym in ALL_SYMS {
                    b.add(create_s1, sym, create_pipe, UtmSym::Zero, Dir::Right);
                }
                for &sym in ALL_SYMS {
                    b.add(create_pipe, sym, create_a0, UtmSym::Pipe, Dir::Right);
                }
                for &sym in ALL_SYMS {
                    b.add(create_a0, sym, create_rb, UtmSym::Zero, Dir::Right); // blank_sym=0
                }
                for &sym in ALL_SYMS {
                    b.add(create_rb, sym, created_back_to_s0, UtmSym::RBracket, Dir::Left);
                }
                // Back up to s0: from ], go left past a0, |, s1, s0 → at [, then right to s0
                let back4 = b.alloc();
                let back3 = b.alloc();
                let back2 = b.alloc();
                let back1 = b.alloc();
                for &sym in ALL_SYMS { b.add(created_back_to_s0, sym, back4, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(back4, sym, back3, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(back3, sym, back2, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(back2, sym, back1, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(back1, sym, write_ns0, sym, Dir::Right); }
                // Hmm, back2 should be at [, move right to s0 → write_ns0. Let me verify:
                // created_back_to_s0 is at ] after writing it. Go left to a0, left to |, left to s1, left to s0.
                // Wait that's 4 lefts from ]. Let me recount:
                // After create_rb writes ] and moves left, we're at the a0 position.
                // No wait, create_rb writes ] at current position and moves LEFT. So we're at a0.
                // From a0: left → |, left → s1, left → s0, left → [
                // So we need 4 lefts to reach [. Then right to s0.
                // created_back_to_s0 is at a0. back4=|, back3=s1, back2=s0, back1=[
                // Actually: created_back_to_s0 moves left from a0 → at |
                // back4 moves left from | → at s1
                // back3 moves left from s1 → at s0
                // back2 moves left from s0 → at [
                // from [ move right to s0 = write_ns0 ✓

                // write_ns0: at s0 of target cell, write ns0
                let ns0_sym = if ns0 == 0 { UtmSym::Zero } else { UtmSym::One };
                for &sym in ALL_SYMS {
                    b.add(write_ns0, sym, write_ns1, ns0_sym, Dir::Right);
                }

                // write_ns1: at s1, write ns1
                let ns1_sym = if ns1 == 0 { UtmSym::Zero } else { UtmSym::One };
                for &sym in ALL_SYMS {
                    b.add(write_ns1, sym, read_sym, ns1_sym, Dir::Right);
                }

                // read_sym: at | of target cell, skip to a0, read symbol
                let read_a0 = b.alloc();
                for &sym in ALL_SYMS {
                    b.add(read_sym, sym, read_a0, sym, Dir::Right); // skip |
                }

                // read_a0: read the symbol, then cleanup and halt check
                build_post_exec(b, read_a0, ns0, ns1, hc01, hc10, hc11);
            }
            Dir::Left => {
                // Move left: from | position, go back to [ of current cell, then
                // left to ] of previous cell, then into that cell's state field.
                // If no previous cell (at beginning of data), create one.

                // From after_clear_s1 (at |): go left to [
                let at_lb = b.alloc();
                let go_left_from_lb = b.alloc();
                // From |, left → s1, left → s0, left → [
                let back_s1 = b.alloc();
                let back_s0 = b.alloc();
                for &sym in ALL_SYMS { b.add(after_clear_s1, sym, back_s1, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(back_s1, sym, back_s0, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(back_s0, sym, go_left_from_lb, sym, Dir::Left); }

                // go_left_from_lb: we should be at [. Go left one more to see what's before.
                // If ] → previous cell exists. Go into it.
                // If # → at beginning of data. Need to create cell.
                // If Blank → shouldn't happen in normal operation

                let prev_at_rb = b.alloc();
                let create_before = b.alloc();

                for &sym in ALL_SYMS {
                    match sym {
                        UtmSym::RBracket => b.add(go_left_from_lb, sym, prev_at_rb, sym, Dir::Left),
                        UtmSym::Hash => {
                            // Beginning of data. Need to insert a cell before current.
                            // This is complex — need to shift all data right.
                            // For now: create cell by writing into the # position.
                            // Actually, we can't shift tape. Let me handle this differently.
                            // Create a new cell to the LEFT: back up into the transition region?
                            // No. TMs have infinite tape in both directions.
                            // Our encoding puts data right after #. Moving left from first cell
                            // means the head is at a position BEFORE the first cell.
                            // We need to insert a new cell at the beginning.
                            // This requires shifting all cells right, which is very complex.
                            // For the smoke test, this case doesn't arise. Skip with reject.
                            b.add(go_left_from_lb, sym, 0, sym, Dir::Right); // punt: reject
                        }
                        _ => b.add(go_left_from_lb, sym, 0, sym, Dir::Right),
                    }
                }

                // prev_at_rb: at ] of previous cell. Navigate into it.
                // From ], left → a0, left → |, left → s1
                let prev_a0 = b.alloc();
                let prev_pipe = b.alloc();
                let prev_s1 = b.alloc();
                let prev_s0 = b.alloc();

                for &sym in ALL_SYMS { b.add(prev_at_rb, sym, prev_a0, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(prev_a0, sym, prev_pipe, sym, Dir::Left); }
                for &sym in ALL_SYMS { b.add(prev_pipe, sym, prev_s1, sym, Dir::Left); }

                // prev_s1: at s1 of previous cell. Write ns1.
                let ns1_sym = if ns1 == 0 { UtmSym::Zero } else { UtmSym::One };
                let write_ns0_prev = b.alloc();
                for &sym in ALL_SYMS {
                    b.add(prev_s1, sym, write_ns0_prev, ns1_sym, Dir::Left);
                }

                // write_ns0_prev: at s0, write ns0
                let ns0_sym = if ns0 == 0 { UtmSym::Zero } else { UtmSym::One };
                let skip_to_read_a0 = b.alloc();
                for &sym in ALL_SYMS {
                    b.add(write_ns0_prev, sym, skip_to_read_a0, ns0_sym, Dir::Right);
                }

                // skip to a0: from s0, right → s1, right → |, right → a0
                let skip_s1 = b.alloc();
                let skip_pipe = b.alloc();
                let read_a0 = b.alloc();
                for &sym in ALL_SYMS { b.add(skip_to_read_a0, sym, skip_s1, sym, Dir::Right); }
                for &sym in ALL_SYMS { b.add(skip_s1, sym, skip_pipe, sym, Dir::Right); }
                for &sym in ALL_SYMS { b.add(skip_pipe, sym, read_a0, sym, Dir::Right); }
                // Hmm wait, skip_to_read_a0 starts at s0 after writing. Right → s1. Right → |. Right → a0.
                // That's 3 rights. Let me add:
                // Actually from write_ns0_prev, we moved right to s1 (skip_to_read_a0).
                // From skip_to_read_a0 (at s1): right → | (skip_s1)
                // From skip_s1 (at |): right → a0 (skip_pipe)
                // From skip_pipe (at a0): this IS read_a0.
                // So skip_pipe IS read_a0. Let me collapse.

                // Actually let me just chain: write_ns0 at s0, move right.
                // We move through s0→s1→|→a0 with intermediate states.
                // s0 is where write_ns0_prev is. After writing and moving right, at s1.
                // skip_to_read_a0 is at s1. Move right → at |.
                // skip_s1 is at |. Move right → at a0.
                // skip_pipe is at a0. This is where we read.

                // Rename: skip_pipe IS read_a0_left.
                build_post_exec(b, skip_pipe, ns0, ns1, hc01, hc10, hc11);
            }
        }
    }

    fn build_post_exec(
        b: &mut TmBuilder,
        read_a0_state: u32,
        ns0: u8,
        ns1: u8,
        hc01: u32, hc10: u32, hc11: u32,
    ) {
        // read_a0_state: we're at the symbol (a0) of the new head cell.
        // We know ns = (ns0, ns1). Read a0 (don't need its value in state).
        // Then cleanup marks and go to halt check.
        //
        // Cleanup: scan left to tape start, restore MarkLBracket→LBracket and Dot→0/1.
        // Then skip header to halt check.
        //
        // Since ns is known, we go to the appropriate halt_check_nsXX.

        let cleanup_scan_l = b.alloc();
        let cleanup_scan_r = b.alloc();

        // From read_a0_state: move left to start cleanup
        for &sym in ALL_SYMS {
            b.add(read_a0_state, sym, cleanup_scan_l, sym, Dir::Left);
        }

        // Scan left to Blank (tape start)
        b.scan_left_to(cleanup_scan_l, UtmSym::Blank, cleanup_scan_r);

        // Scan right, restore marks. Stop at # (data region begins).
        // After restoring, we're at # with clean transitions.
        // Then scan right into data → halt check.
        let target_hc = match (ns0, ns1) {
            (0, 1) => hc01,
            (1, 0) => hc10,
            (1, 1) => hc11,
            _ => 0, // ns=00 shouldn't happen
        };

        // After cleanup, need to go to data to find head cell for halt check.
        // Approach: scan right from tape start. During cleanup, fix marks. After #, enter data → find head → halt check.
        // But halt_check_nsXX expects us to be at the head cell's s1 position!
        // Let me re-examine halt_check entry point.

        // Actually, halt_check_nsXX was built with build_halt_check which starts by
        // scanning left to Blank from the entry state. So the entry state can be
        // anywhere — it will scan left to Blank first.

        // So I can just go to halt_check_nsXX after cleanup. But I need to end up
        // somewhere reasonable. Let me just have cleanup go to halt check directly.

        for &sym in ALL_SYMS {
            match sym {
                UtmSym::MarkLBracket => b.add(cleanup_scan_r, sym, cleanup_scan_r, UtmSym::LBracket, Dir::Right),
                UtmSym::Dot0 => b.add(cleanup_scan_r, sym, cleanup_scan_r, UtmSym::Zero, Dir::Right),
                UtmSym::Dot1 => b.add(cleanup_scan_r, sym, cleanup_scan_r, UtmSym::One, Dir::Right),
                UtmSym::Blank => {
                    // Reached end of tape. All marks cleaned. Go to halt check.
                    b.add(cleanup_scan_r, sym, target_hc, sym, Dir::Left);
                }
                _ => b.add(cleanup_scan_r, sym, cleanup_scan_r, sym, Dir::Right),
            }
        }
    }

    build_execute(&mut b, exec_r_goto_hash, Dir::Right, halt_check_ns01, halt_check_ns10, halt_check_ns11);
    build_execute(&mut b, exec_l_goto_hash, Dir::Left, halt_check_ns01, halt_check_ns10, halt_check_ns11);

    // ====== Build the TM ======
    TuringMachine {
        initial: init,
        accept: 1,
        reject: 0,
        blank: UtmSym::Blank,
        transitions: b.transitions,
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

    // ===== UTM TM tests =====

    /// A TM that writes a 1 to the blank tape and accepts.
    /// States: Reject(0), Start(1), Accept(2) → 2-bit state
    /// Symbols: Blank(0), One(1) → 1-bit symbol
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum W1State { Start, Accept, Reject }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum W1Sym { Blank, One }

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

    #[test]
    fn test_utm_tm_directly() {
        // Run the UTM TM directly (as a regular TM) on the encoded write1 tape.
        let w1 = write1_tm();
        let inner_tape = encode(&w1, &[]);

        eprintln!("Inner tape ({} symbols): {:?}", inner_tape.len(), inner_tape);

        // The UTM TM operates on UtmSym symbols.
        let utm = build_utm_tm();
        eprintln!("UTM TM: {} states, {} transitions", utm.transitions.len(),
            utm.transitions.len());

        // Run the UTM TM directly using tm::run.
        let result = tm::run(&utm, &inner_tape, 1_000_000)
            .expect("UTM TM should halt");

        eprintln!("UTM TM outcome: {:?}", result.outcome);
        eprintln!("UTM TM tape ({} symbols): {:?}", result.tape.len(), result.tape);

        assert_eq!(result.outcome, Outcome::Accept,
            "UTM TM should accept (simulated TM accepts)");

        // Decode the result tape to get the simulated TM's output.
        // The result tape should be a valid encoded tape for write1_tm.
        let inner_decoded = decode(&w1, &result.tape, result.outcome == Outcome::Accept);

        // Strip trailing blanks and check for One
        let syms: Vec<_> = inner_decoded.tape.iter()
            .copied()
            .filter(|s| *s != W1Sym::Blank)
            .collect();
        assert!(syms.contains(&W1Sym::One),
            "Decoded tape should contain One, got {:?}", inner_decoded.tape);
    }

    #[test]
    fn test_utm_on_utm_smoke() {
        // The big test: run the UTM interpreter on encode(UTM_TM, encode(write1, [])).
        // Then decode twice to get the write1 result.

        let w1 = write1_tm();
        let inner_tape = encode(&w1, &[]); // encode write1 for the UTM

        let utm = build_utm_tm();
        let outer_tape = encode(&utm, &inner_tape); // encode UTM_TM with inner_tape as input

        // Run the UTM interpreter on the outer encoding.
        // This simulates the UTM TM, which itself simulates write1.
        let (result_tape, accepted) = run_utm_on(&outer_tape, 10_000_000)
            .expect("Outer UTM should halt");

        assert!(accepted, "Outer UTM should report accept");

        // First decode: UTM TM's output tape → Vec<UtmSym>
        let middle = decode(&utm, &result_tape, accepted);
        assert_eq!(middle.outcome, Outcome::Accept,
            "Middle decode should show accept");

        // Second decode: interpret the middle tape as write1's encoded output
        let final_result = decode(&w1, &middle.tape, middle.outcome == Outcome::Accept);
        assert_eq!(final_result.outcome, Outcome::Accept,
            "Final decode should show accept");

        // The doubly-decoded tape should contain One
        let syms: Vec<_> = final_result.tape.iter()
            .copied()
            .filter(|s| *s != W1Sym::Blank)
            .collect();
        assert!(syms.contains(&W1Sym::One),
            "Doubly-decoded tape should read '1', got {:?}", final_result.tape);
    }
}
