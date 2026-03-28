use crate::utm::{make_utm_spec, MyUtmSpec, MyUtmSpecOptimizationHints};

/// Optimization hints for UTM rule ordering.
/// Rules listed here should be encoded last (most-used rules at the end)
/// because the UTM scans rules right-to-left.
/// Generated from src/my-utm-spec-transition-optimization-hints.ts
pub fn make_my_utm_self_optimization_hints() -> MyUtmSpecOptimizationHints<MyUtmSpec> {
    MyUtmSpecOptimizationHints::guess(&make_utm_spec()) // TODO
}
