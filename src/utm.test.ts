import { describe, expect, it } from "vitest";
import { makeInitSnapshot, run, step } from "./types";
import { utmSpec } from "./utm";
import { acceptImmediatelySpec, checkPalindromeSpec, flipBitsSpec, rejectImmediatelySpec } from "./toy-machines";
import { isDeepStrictEqual } from "node:util";

describe("decode", () => {
  it("inverts encode", () => {
    const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "a"]);

    const encoded = utmSpec.encode(tm);
    const decoded = utmSpec.decode(tm.spec, encoded);
    expect(decoded).toEqual(tm);

    while (step(tm) === "continue") {
      const encoded = utmSpec.encode(tm);
      const decoded = utmSpec.decode(tm.spec, encoded);
      expect(decoded).toEqual(tm);
    }
  });
});

describe('rules', () => {
  it("decodes to original snapshot, then undefined, then stepped snapshot", () => {
    const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "a"]);
    const utm = makeInitSnapshot(utmSpec, utmSpec.encode(tm));

    const snap0 = utmSpec.decode(tm.spec, utm.tape);
    expect(snap0).toEqual(tm);
    step(tm);

    while (isDeepStrictEqual(snap0, utmSpec.decode(tm.spec, utm.tape))) {
      expect(step(utm)).toBe("continue");
    }
    while (utmSpec.decode(tm.spec, utm.tape) === undefined) {
      expect(step(utm)).toBe("continue");
    }

    const snap1 = utmSpec.decode(tm.spec, utm.tape);
    expect(snap1).not.toEqual(snap0);
    expect(snap1).toEqual(tm);

    while (isDeepStrictEqual(snap1, utmSpec.decode(tm.spec, utm.tape))) {
      expect(step(utm)).toBe("continue");
    }
    while (utmSpec.decode(tm.spec, utm.tape) === undefined) {
      expect(step(utm)).toBe("continue");
    }

    const snap2 = utmSpec.decode(tm.spec, utm.tape);
    expect(snap2).not.toEqual(snap1);
    expect(snap2).toEqual(tm);
  });

  it('terminates with the same status as the simulated machine', () => {
    expect(run(makeInitSnapshot(acceptImmediatelySpec, []))).toBe("accept");
    expect(run(makeInitSnapshot(rejectImmediatelySpec, []))).toBe("reject");
  });

  it('terminates with the correct decoded tape', () => {
    const tm = makeInitSnapshot(flipBitsSpec, ["0", "1", "0", "1", "1"]);
    const utm = makeInitSnapshot(utmSpec, utmSpec.encode(tm));

    expect(run(tm)).toBe("accept");
    expect(tm.tape).toEqual(["1", "0", "1", "0", "0"]);

    expect(run(utm)).toBe("accept");

    expect(utmSpec.decode(tm.spec, utm.tape)).toEqual(tm);
  });
});
