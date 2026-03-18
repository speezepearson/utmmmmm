import { type TuringMachineSpec } from "./types";

export const write1sForeverSpec: TuringMachineSpec<"init", "_" | "1"> = {
  allStates: ["init"] as const,
  allSymbols: ["_", "1"] as const,
  initial: "init",
  blank: "_",
  rules: {
    init: {
      _: { type: "step", newState: "init", newSymbol: "1", dir: "R" },
      "1": { type: "step", newState: "init", newSymbol: "1", dir: "R" },
    },
  },
};

export const acceptImmediatelySpec: TuringMachineSpec<"init", "_"> = {
  allStates: ["init"] as const,
  allSymbols: ["_"] as const,
  initial: "init",
  blank: "_",
  rules: {
    init: {
      _: { type: "accept" },
    },
  },
};

type PalState =
  | "start"
  | `going_right_a` // erased left char, scanning right (remembers which letter)
  | `going_right_b`
  | `check_right_a` // at right boundary, about to compare (remembers which letter)
  | `check_right_b`
  | "going_left"; // matched pair, scanning back left

type PalSymbol = "_" | "a" | "b";

export const checkPalindromeSpec: TuringMachineSpec<
  PalState,
  PalSymbol
> = {
    allStates: [
        "start",
        "going_right_a",
        "going_right_b",
        "check_right_a",
        "check_right_b",
        "going_left",
      ] as const,
    allSymbols: ["_", "a", "b"] as const,
    initial: "start",
    blank: "_",
    rules: {
        // ── start ──────────────────────────────────────────────────────────────
        // All blanks consumed → accept (even-length palindrome fully processed).
        // Otherwise erase the leftmost character and remember it in the state.
        start: {
            _: { type: "accept" },
            a: { type: "step", newState: "going_right_a", newSymbol: "_", dir: "R" },
            b: { type: "step", newState: "going_right_b", newSymbol: "_", dir: "R" },
        },
        // ── going_right_X ──────────────────────────────────────────────────────
        // Pass over all remaining characters rightward; on blank, step back left.
        going_right_a: {
            _: { type: "step", newState: "check_right_a", newSymbol: "_", dir: "L" },
            a: { type: "step", newState: "going_right_a", newSymbol: "_", dir: "R" },
            b: { type: "step", newState: "going_right_b", newSymbol: "_", dir: "R" },
        },
        going_right_b: {
            _: { type: "step", newState: "check_right_b", newSymbol: "_", dir: "L" },
            a: { type: "step", newState: "going_right_a", newSymbol: "_", dir: "R" },
            b: { type: "step", newState: "going_right_b", newSymbol: "_", dir: "R" },
        },
        // ── check_right_X ──────────────────────────────────────────────────────
        // Now at the rightmost remaining character (or blank if only X was left).
        // Blank  → only the middle character existed → accept (odd palindrome).
        // Match  → erase, head left to find new left boundary.
        // Differ → not a palindrome → reject.
        check_right_a: {
            _: { type: "accept" },
            a: { type: "step", newState: "going_left", newSymbol: "_", dir: "L" },
            b: { type: "reject" },
        },
        check_right_b: {
            _: { type: "accept" },
            a: { type: "reject" },
            b: { type: "step", newState: "going_left", newSymbol: "_", dir: "L" },
        },
        // ── going_left ─────────────────────────────────────────────────────────
        // Scan left over all remaining characters until we hit a blank (the
        // erased left edge), then step right to place the head on the new
        // leftmost unprocessed character.
        going_left: {
            _: { type: "step", newState: "start", newSymbol: "_", dir: "R" },
            a: { type: "step", newState: "going_left", newSymbol: "_", dir: "L" },
            b: { type: "step", newState: "going_left", newSymbol: "_", dir: "L" },
        },
    },
  };