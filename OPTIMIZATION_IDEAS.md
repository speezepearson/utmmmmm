# UTM Self-Simulation Optimization Ideas

## State scratchpad propagation (not yet implemented, empirical question)

Instead of scanning between the active rule and the STATE section for each
bit comparison, copy the current state into a scratchpad at the beginning of
the rule, and compare the rule's state predicate against that local scratchpad.

When a rule fails to match, copy its scratchpad forward to the next rule's
scratchpad, propagating the current state along the rules section.

### Cost/benefit analysis:
- **Cost per simulated step**: O(2 * stateLen * ruleWidth * numRulesConsidered)
  for propagating the state through each rule's scratchpad
- **Savings per simulated step**: O(2 * ruleWidth * 0.5 * numRulesConsidered^2)
  since without this optimization, the UTM scans across longer and longer
  suffixes of the rules section to reach the STATE section
- **Additional cost**: The scratchpad area increases the number of symbols per
  rule, meaning more UTM symbols to scan through, and in self-simulation the
  outer machine has more states to consider (more rules per state due to longer
  scan sequences), partially offsetting the savings
- **Verdict**: Empirical question whether this helps. The quadratic savings in
  numRulesConsidered could dominate for large rule sets, but the increased
  per-rule width and expanded outer-machine rule count work against it.

## HEADCACHE section

Add a HEADCACHE section (copy of the symbol under the simulated head) adjacent
to STATE, so symbol comparisons don't need to traverse to the tape section.

- Cost per head move: O(symBits * tapePos) to copy new symbol to cache
- Savings: ~16 * O(tapePos) per step (estimated ~20 sym-bit comparisons
  per step, minus 4 bits of copy cost)
- Net: likely positive, especially as tape grows

## Prefix-optimized binary code assignments

Assign binary codes to states (and symbols) so that frequently-compared
and-mismatched pairs differ in the earliest possible bit position. This
minimizes average comparison steps before mismatch during rule search.
Pure encoding-time optimization, no UTM changes needed.
