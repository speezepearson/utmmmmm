// ════════════════════════════════════════════════════════════════════
// Prefix-optimized binary code assignment for UTM state/symbol encoding
//
// When the UTM compares state (or symbol) bits during rule search, it
// goes MSB to LSB and bails on the first mismatch. By assigning binary
// codes so that frequently-confused pairs differ in early bit positions,
// we minimize the average number of bit comparisons per rule check.
// ════════════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::hash::Hash;

use crate::tm::{Dir, TuringMachineSpec};

/// Compute an optimal ordering of states for binary encoding.
///
/// Returns a Vec where `result[i]` is the state that should get binary code `i`.
/// States that are frequently confused during rule search will differ in their
/// most significant bits.
///
/// `ordered_rules` should be in the same order they'll appear on tape (last = checked first).
pub fn compute_optimal_state_order<S, Y>(
    states: &[S],
    ordered_rules: &[(S, Y, S, Y, Dir)],
) -> Vec<S>
where
    S: Copy + Eq + Hash,
    Y: Copy + Eq + Hash,
{
    let n = states.len();
    if n <= 2 {
        return states.to_vec();
    }

    let st_idx: HashMap<S, usize> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, i))
        .collect();

    let weights = compute_confusion_weights(n, &st_idx, ordered_rules);
    let indices: Vec<usize> = (0..n).collect();
    let ordered_indices = recursive_bisection(&indices, &weights);

    ordered_indices.iter().map(|&i| states[i]).collect()
}

/// Compute an optimal ordering of symbols for binary encoding.
///
/// Similar to state ordering, but the confusion matrix is based on symbol
/// comparisons (which only happen after state comparison succeeds).
pub fn compute_optimal_symbol_order<S, Y>(
    states: &[S],
    symbols: &[Y],
    ordered_rules: &[(S, Y, S, Y, Dir)],
) -> Vec<Y>
where
    S: Copy + Eq + Hash,
    Y: Copy + Eq + Hash,
{
    let n = symbols.len();
    if n <= 2 {
        return symbols.to_vec();
    }

    let st_idx: HashMap<S, usize> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, i))
        .collect();
    let sym_idx: HashMap<Y, usize> = symbols
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, i))
        .collect();

    let weights = compute_symbol_confusion_weights(
        states.len(),
        symbols.len(),
        &st_idx,
        &sym_idx,
        ordered_rules,
    );
    let indices: Vec<usize> = (0..n).collect();
    let ordered_indices = recursive_bisection(&indices, &weights);

    ordered_indices.iter().map(|&i| symbols[i]).collect()
}

/// Compute the state confusion weight matrix.
///
/// `weights[i][j]` = number of times state i is the current state and a rule
/// with state j is checked (and fails on state comparison) before finding
/// the matching rule, summed over all (state, symbol) pairs.
///
/// Rules are scanned right-to-left, so rule at position N-1 is checked first.
fn compute_confusion_weights<S, Y>(
    n_states: usize,
    st_idx: &HashMap<S, usize>,
    ordered_rules: &[(S, Y, S, Y, Dir)],
) -> Vec<Vec<f64>>
where
    S: Copy + Eq + Hash,
    Y: Copy + Eq + Hash,
{
    let mut weights = vec![vec![0.0f64; n_states]; n_states];
    let n_rules = ordered_rules.len();

    // For each rule at position `pos`, it represents the transition for (state, symbol).
    // The UTM scans right-to-left, so rules at higher positions are checked first.
    // For the (state, symbol) pair of this rule, all rules at positions > pos
    // are checked before this one, and their state comparisons contribute confusion.
    for pos in 0..n_rules {
        let current_state = ordered_rules[pos].0;
        let cs_idx = st_idx[&current_state];

        // All rules checked before this one (higher positions = checked earlier)
        for check_pos in (pos + 1)..n_rules {
            let checked_state = ordered_rules[check_pos].0;
            let cks_idx = st_idx[&checked_state];
            if cks_idx != cs_idx {
                weights[cs_idx][cks_idx] += 1.0;
            }
        }
    }

    // Symmetrize: we care about first_diff_bit(code[i], code[j]) which is symmetric,
    // so combine both directions.
    for i in 0..n_states {
        for j in (i + 1)..n_states {
            let sym = weights[i][j] + weights[j][i];
            weights[i][j] = sym;
            weights[j][i] = sym;
        }
    }

    weights
}

/// Compute the symbol confusion weight matrix.
///
/// Symbol comparisons only happen after state comparison succeeds.
/// For each (state, symbol) pair, after finding a rule that matches the state,
/// the UTM compares symbols. Rules with matching state but wrong symbol
/// contribute to symbol confusion.
fn compute_symbol_confusion_weights<S, Y>(
    _n_states: usize,
    n_symbols: usize,
    st_idx: &HashMap<S, usize>,
    sym_idx: &HashMap<Y, usize>,
    ordered_rules: &[(S, Y, S, Y, Dir)],
) -> Vec<Vec<f64>>
where
    S: Copy + Eq + Hash,
    Y: Copy + Eq + Hash,
{
    let mut weights = vec![vec![0.0f64; n_symbols]; n_symbols];
    let n_rules = ordered_rules.len();

    // For each rule at position `pos` with (state s, symbol y):
    // Rules at positions > pos that have the same state s but different symbol y'
    // contribute symbol confusion between y and y'.
    for pos in 0..n_rules {
        let (cur_st, cur_sym, _, _, _) = ordered_rules[pos];
        let cur_st_idx = st_idx[&cur_st];
        let cur_sym_idx = sym_idx[&cur_sym];

        for check_pos in (pos + 1)..n_rules {
            let (chk_st, chk_sym, _, _, _) = ordered_rules[check_pos];
            let chk_st_idx = st_idx[&chk_st];
            let chk_sym_idx = sym_idx[&chk_sym];

            // Symbol comparison only happens when state matches
            if chk_st_idx == cur_st_idx && chk_sym_idx != cur_sym_idx {
                weights[cur_sym_idx][chk_sym_idx] += 1.0;
            }
        }
    }

    // Symmetrize
    for i in 0..n_symbols {
        for j in (i + 1)..n_symbols {
            let sym = weights[i][j] + weights[j][i];
            weights[i][j] = sym;
            weights[j][i] = sym;
        }
    }

    weights
}

/// Recursively bisect items to assign binary codes.
///
/// Returns items ordered such that items in the first half get MSB=0 and
/// items in the second half get MSB=1. Within each half, the same property
/// holds for the next bit, recursively.
///
/// At each level, maximizes cross-group confusion weight so that
/// high-weight pairs differ at the current (most significant) bit.
fn recursive_bisection(items: &[usize], weights: &[Vec<f64>]) -> Vec<usize> {
    let n = items.len();
    if n <= 1 {
        return items.to_vec();
    }
    if n == 2 {
        return items.to_vec();
    }

    let (group_a, group_b) = max_bisection(items, weights);
    let mut result = recursive_bisection(&group_a, weights);
    result.extend(recursive_bisection(&group_b, weights));

    // Pad to next power of 2 if needed (codes must be uniform width)
    result
}

/// Find a balanced bisection of `items` that maximizes cross-group weight.
/// Uses Kernighan-Lin style iterative improvement.
fn max_bisection(items: &[usize], weights: &[Vec<f64>]) -> (Vec<usize>, Vec<usize>) {
    let n = items.len();
    let half = n / 2;

    // Initial split: sort by total confusion weight, alternate assignment.
    // This puts the "heaviest" nodes in different groups.
    let mut sorted: Vec<(usize, f64)> = items
        .iter()
        .map(|&i| {
            let total: f64 = items.iter().map(|&j| weights[i][j]).sum();
            (i, total)
        })
        .collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut in_a = vec![false; weights.len()]; // indexed by original item id
    let mut group_a: Vec<usize> = Vec::with_capacity(half);
    let mut group_b: Vec<usize> = Vec::with_capacity(n - half);

    for (idx, &(item, _)) in sorted.iter().enumerate() {
        if idx % 2 == 0 && group_a.len() < half {
            group_a.push(item);
            in_a[item] = true;
        } else if group_b.len() < n - half {
            group_b.push(item);
        } else {
            group_a.push(item);
            in_a[item] = true;
        }
    }

    // Kernighan-Lin iterative improvement
    loop {
        let mut best_gain = 0.0f64;
        let mut best_swap: Option<(usize, usize)> = None;

        for (ai, &a) in group_a.iter().enumerate() {
            for (bi, &b) in group_b.iter().enumerate() {
                let gain = swap_gain(a, b, &in_a, items, weights);
                if gain > best_gain + 1e-10 {
                    best_gain = gain;
                    best_swap = Some((ai, bi));
                }
            }
        }

        if let Some((ai, bi)) = best_swap {
            let a = group_a[ai];
            let b = group_b[bi];
            in_a[a] = false;
            in_a[b] = true;
            group_a[ai] = b;
            group_b[bi] = a;
        } else {
            break;
        }
    }

    (group_a, group_b)
}

/// Compute the gain from swapping item `a` (in group A) with item `b` (in group B).
/// Gain = increase in cross-group weight from the swap.
fn swap_gain(
    a: usize,
    b: usize,
    in_a: &[bool],
    items: &[usize],
    weights: &[Vec<f64>],
) -> f64 {
    let mut gain = 0.0f64;

    for &item in items {
        if item == a || item == b {
            continue;
        }

        if in_a[item] {
            // item is in A.
            // Before swap: a-item is intra-A (0), b-item is cross (weights[b][item])
            // After swap:  b-item is intra-A (0), a-item is cross (weights[a][item])
            gain += weights[a][item] - weights[b][item];
        } else {
            // item is in B.
            // Before swap: b-item is intra-B (0), a-item is cross (weights[a][item])
            // After swap:  a-item is intra-B (0), b-item is cross (weights[b][item])
            gain += weights[b][item] - weights[a][item];
        }
    }

    // Also account for the a-b pair itself:
    // Before: a in A, b in B -> cross (weights[a][b])
    // After:  b in A, a in B -> still cross (weights[a][b])
    // No change for the a-b pair.

    gain
}

/// Convenience: compute optimal orderings for a TuringMachineSpec given a rule order.
pub fn compute_optimal_orders<Guest: TuringMachineSpec>(
    spec: &Guest,
    rule_order: &[(Guest::State, Guest::Symbol)],
) -> (Vec<Guest::State>, Vec<Guest::Symbol>) {
    let states: Vec<Guest::State> = spec.iter_states().collect();
    let symbols: Vec<Guest::Symbol> = spec.iter_symbols().collect();

    // Build the full ordered rule list from the rule_order hints
    let all_rules: Vec<(Guest::State, Guest::Symbol, Guest::State, Guest::Symbol, Dir)> =
        spec.iter_rules().collect();

    let rule_order_set: std::collections::HashSet<(Guest::State, Guest::Symbol)> =
        rule_order.iter().copied().collect();

    let mut ordered_rules: Vec<(Guest::State, Guest::Symbol, Guest::State, Guest::Symbol, Dir)> =
        Vec::new();

    // Rules not in rule_order come first
    for &(st, sym, nst, nsym, dir) in &all_rules {
        if !rule_order_set.contains(&(st, sym)) {
            ordered_rules.push((st, sym, nst, nsym, dir));
        }
    }
    // Then rules in rule_order, in the specified order
    for &(lst, lsym) in rule_order {
        if let Some(&(st, sym, nst, nsym, dir)) =
            all_rules.iter().find(|(st, sym, _, _, _)| *st == lst && *sym == lsym)
        {
            ordered_rules.push((st, sym, nst, nsym, dir));
        }
    }

    let state_order = compute_optimal_state_order(&states, &ordered_rules);
    let symbol_order = compute_optimal_symbol_order(&states, &symbols, &ordered_rules);

    (state_order, symbol_order)
}
