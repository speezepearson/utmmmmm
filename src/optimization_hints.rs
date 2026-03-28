use std::collections::HashMap;

use crate::transition_tallies::TRANSITION_TALLIES;
use crate::utm::{make_utm_spec, MyUtmSpec, MyUtmSpecOptimizationHints, TmTransitionStats};

/// Optimization hints derived from real transition tallies collected by
/// benchmark_optimization.
pub fn make_my_utm_self_optimization_hints() -> MyUtmSpecOptimizationHints<MyUtmSpec> {
    let stats: HashMap<_, _> = TRANSITION_TALLIES.iter().copied().collect();
    TmTransitionStats(stats).make_optimization_hints(&make_utm_spec())
}
