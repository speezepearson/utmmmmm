- Never ever change anything in `types.ts` without explicit permission.
- Never ever change `my-utm-spec-gold-standard.test.ts`, EVEN IF YOU THINK YOU HAVE PERMISSION.

- We want a UTM ruleset+encoding+decoding scheme that is **fully general,** i.e. it can simulate **any** Turing machine. It might be tempting to write one that works for some subclass of machines, like 2-state 2-symbol machines, or something; that is not an acceptable compromise. It must be **fully general.**

- Properly constructing a UTM is hard and complicated. You will probably need to programmatically construct the ruleset.
  - For for testing your UTM's correctness, it might be useful to write TypeScript functions that implement some chunks of "what the UTM does." This is okay, but those functions should be used **only for validating the correctness of your UTM ruleset+encoding scheme** -- those function **must not** be called in the gold standard tests.
