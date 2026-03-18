import { describe, expect, it } from "vitest";
import { makeInitSnapshot, run, step } from "./types";
import { acceptImmediatelySpec, checkPalindromeSpec, flipBitsSpec, rejectImmediatelySpec } from "./toy-machines";

describe('acceptImmediatelySpec', () => {
  it('terminates with accept', () => {
    expect(step(makeInitSnapshot(acceptImmediatelySpec, []))).toBe("accept");
  });
});

describe('rejectImmediatelySpec', () => {
  it('terminates with reject', () => {
    expect(step(makeInitSnapshot(rejectImmediatelySpec, []))).toBe("reject");
  });
});

describe('flipBitsSpec', () => {
  it('flips bits', () => {
    const tm = makeInitSnapshot(flipBitsSpec, ["0", "1", "0", "1", "1"]);
    expect(run(tm)).toBe("accept");
    expect(tm.tape).toEqual(["1", "0", "1", "0", "0"]);
  });
});

describe('checkPalindromeSpec', () => {
  it('accepts an even-length palindrome', () => {
    const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "a"]);
    expect(run(tm)).toBe("accept");
  });
  it('accepts an odd-length palindrome', () => {
    const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "b", "a"]);
    expect(run(tm)).toBe("accept");
  });
  it('rejects a non-palindrome', () => {
    const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "a", "b"]);
    expect(run(tm)).toBe("reject");
  });
});
