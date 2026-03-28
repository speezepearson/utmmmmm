use std::collections::{HashMap, HashSet};

use utmmmmm::gen_utm::UtmSpec as _;
use utmmmmm::tm::{Dir, RunningTMStatus, RunningTuringMachine, TuringMachineSpec};
use utmmmmm::toy_machines::*;
use utmmmmm::utm::{make_utm_spec, State, Symbol};

struct IntervalStats {
    appearances: HashMap<(State, Symbol), usize>,
    exactly_once: HashMap<(State, Symbol), usize>,
    total_intervals: usize,
    ever_had_xy: HashSet<(State, Symbol)>,
}

fn analyze_machine(
    name: &str,
    utm_spec: &utmmmmm::utm::MyUtmSpec,
    encoded: Vec<Symbol>,
) -> IntervalStats {
    let head_move_caret_states = [State::MrSkipCell, State::MlMark, State::MrExtToBlank];

    let mut utm_tm = RunningTuringMachine::new(utm_spec);
    utm_tm.tape = encoded;

    let mut transition_counts: HashMap<(State, Symbol), usize> = HashMap::new();
    let mut had_xy_this_interval: HashSet<(State, Symbol)> = HashSet::new();
    let mut caret_number = 0;

    let mut interval_appearances: HashMap<(State, Symbol), usize> = HashMap::new();
    let mut interval_exactly_once: HashMap<(State, Symbol), usize> = HashMap::new();
    let mut ever_had_xy: HashSet<(State, Symbol)> = HashSet::new();

    // Track X/Y count incrementally
    let mut xy_count: usize = utm_tm
        .tape
        .iter()
        .filter(|&&s| s == Symbol::X || s == Symbol::Y)
        .count();

    let max_steps = 200_000_000;

    for _ in 0..max_steps {
        if utm_tm.pos >= utm_tm.tape.len() {
            utm_tm.tape.resize(utm_tm.pos + 1, utm_spec.blank());
        }

        let state = utm_tm.state;
        let sym = utm_tm.tape[utm_tm.pos];

        if let Some((ns, new_sym, dir)) = utm_spec.get_transition(state, sym) {
            *transition_counts.entry((state, sym)).or_insert(0) += 1;

            if xy_count > 0 {
                had_xy_this_interval.insert((state, sym));
            }

            // Update xy_count: old symbol removed, new symbol added
            if sym == Symbol::X || sym == Symbol::Y {
                xy_count -= 1;
            }
            if new_sym == Symbol::X || new_sym == Symbol::Y {
                xy_count += 1;
            }

            // Apply the transition manually
            utm_tm.state = ns;
            utm_tm.tape[utm_tm.pos] = new_sym;
            utm_tm.pos = match dir {
                Dir::Left => utm_tm.pos.saturating_sub(1),
                Dir::Right => utm_tm.pos + 1,
            };

            if new_sym == Symbol::Caret && head_move_caret_states.contains(&state) {
                caret_number += 1;

                for (&key, &count) in &transition_counts {
                    *interval_appearances.entry(key).or_insert(0) += 1;
                    if count == 1 {
                        *interval_exactly_once.entry(key).or_insert(0) += 1;
                    }
                }
                for &key in &had_xy_this_interval {
                    ever_had_xy.insert(key);
                }
                transition_counts.clear();
                had_xy_this_interval.clear();
            }
        } else {
            // Halted
            if utm_spec.is_accepting(state) {
                break;
            } else {
                break;
            }
        }
    }

    // Flush final interval's XY data
    for &key in &had_xy_this_interval {
        ever_had_xy.insert(key);
    }

    eprintln!(
        "{}: {} head-move carets",
        name, caret_number,
    );

    IntervalStats {
        appearances: interval_appearances,
        exactly_once: interval_exactly_once,
        total_intervals: caret_number,
        ever_had_xy,
    }
}

fn main() {
    let utm_spec = make_utm_spec();

    // 1. Flip bits [0, 1]
    let mut tm1 = RunningTuringMachine::new(&*FLIP_BITS_SPEC);
    tm1.tape = vec![FlipBitsSymbol::Zero, FlipBitsSymbol::One];
    let enc1 = utm_spec.encode(&tm1);

    // 2. Flip bits [0, 1, 1, 0, 1]
    let mut tm2 = RunningTuringMachine::new(&*FLIP_BITS_SPEC);
    tm2.tape = vec![
        FlipBitsSymbol::Zero,
        FlipBitsSymbol::One,
        FlipBitsSymbol::One,
        FlipBitsSymbol::Zero,
        FlipBitsSymbol::One,
    ];
    let enc2 = utm_spec.encode(&tm2);

    // 3. Double Xs [$, X, X, X]
    let mut tm3 = RunningTuringMachine::new(&*DOUBLE_X_SPEC);
    tm3.tape = vec![
        DoubleXSymbol::Dollar,
        DoubleXSymbol::X,
        DoubleXSymbol::X,
        DoubleXSymbol::X,
    ];
    let enc3 = utm_spec.encode(&tm3);

    // 4. UTM encoding of accept_immediately (UTM simulating a UTM)
    let mut tm_acc = RunningTuringMachine::new(&*ACCEPT_IMMEDIATELY_SPEC);
    tm_acc.tape = vec![AccImmSymbol::Blank];
    let inner_encoded = utm_spec.encode(&tm_acc);
    let mut tm4 = RunningTuringMachine::new(&utm_spec);
    tm4.tape = inner_encoded;
    let enc4 = utm_spec.encode(&tm4);

    let stats = [
        analyze_machine("flip_bits(01)", &utm_spec, enc1),
        analyze_machine("flip_bits(01101)", &utm_spec, enc2),
        analyze_machine("double_xs($XXX)", &utm_spec, enc3),
        analyze_machine("utm(accept_imm)", &utm_spec, enc4),
    ];

    // Find transitions that are exactly-once in every interval of EVERY machine,
    // and never occur while X/Y is on tape
    let all_keys: HashSet<(State, Symbol)> = stats
        .iter()
        .flat_map(|s| s.appearances.keys().copied())
        .collect();

    let mut candidates: Vec<_> = all_keys
        .into_iter()
        .filter(|key| {
            stats.iter().all(|s| {
                let total = s.total_intervals;
                if total == 0 {
                    return false;
                }
                !s.ever_had_xy.contains(key)
                    && s.appearances.get(key) == Some(&total)
                    && s.exactly_once.get(key) == Some(&total)
            })
        })
        .collect();
    candidates.sort_by_key(|&(st, _)| format!("{:?}", st));

    println!();
    println!("=== Transitions exactly-once in every interval of ALL machines, never with X/Y on tape ===");
    for (st, sy) in &candidates {
        println!("  ({:?}, {:?})", st, sy);
    }
    println!("  ({} transitions)", candidates.len());
}
