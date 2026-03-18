/**
 * DO NOT MODIFY THIS FILE.
 */

import { describe, expect, it } from "vitest";
import { makeInitSnapshot, run, step } from "./types";
import { myUtmSpec } from "./my-utm-spec";
import { acceptImmediatelySpec, checkPalindromeSpec, flipBitsSpec, rejectImmediatelySpec } from "./toy-machines";
import { isDeepStrictEqual } from "node:util";

describe('myUtmSpec gold standard tests', () => {
  describe("decode", () => {
    it("inverts encode", () => {
      const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "a"]);

      const encoded = myUtmSpec.encode(tm);
      const decoded = myUtmSpec.decode(tm.spec, encoded);
      expect(decoded).toEqual(tm);

      while (step(tm) === "continue") {
        const encoded = myUtmSpec.encode(tm);
        const decoded = myUtmSpec.decode(tm.spec, encoded);
        expect(decoded).toEqual(tm);
      }
    });
  });

  describe('rules', () => {
    it("decodes to original snapshot, then undefined, then stepped snapshot", () => {
      const tm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "b", "a"]);
      const utm = makeInitSnapshot(myUtmSpec, myUtmSpec.encode(tm));

      const snap0 = myUtmSpec.decode(tm.spec, utm.tape);
      expect(snap0).toEqual(tm);
      step(tm);

      while (isDeepStrictEqual(snap0, myUtmSpec.decode(tm.spec, utm.tape))) {
        expect(step(utm)).toBe("continue");
      }
      while (myUtmSpec.decode(tm.spec, utm.tape) === undefined) {
        expect(step(utm)).toBe("continue");
      }

      const snap1 = myUtmSpec.decode(tm.spec, utm.tape);
      expect(snap1).not.toEqual(snap0);
      expect(snap1).toEqual(tm);

      while (isDeepStrictEqual(snap1, myUtmSpec.decode(tm.spec, utm.tape))) {
        expect(step(utm)).toBe("continue");
      }
      while (myUtmSpec.decode(tm.spec, utm.tape) === undefined) {
        expect(step(utm)).toBe("continue");
      }

      const snap2 = myUtmSpec.decode(tm.spec, utm.tape);
      expect(snap2).not.toEqual(snap1);
      expect(snap2).toEqual(tm);
    });

    it('terminates with the same status as the simulated machine', () => {
      expect(run(makeInitSnapshot(acceptImmediatelySpec, []))).toBe("accept");
      expect(run(makeInitSnapshot(rejectImmediatelySpec, []))).toBe("reject");
    });

    it('terminates with the correct decoded tape', () => {
      const tm = makeInitSnapshot(flipBitsSpec, ["0", "1", "0", "1", "1"]);
      const utm = makeInitSnapshot(myUtmSpec, myUtmSpec.encode(tm));

      expect(run(tm)).toBe("accept");
      expect(tm.tape).toEqual(["1", "0", "1", "0", "0"]);

      expect(run(utm)).toBe("accept");

      expect(myUtmSpec.decode(tm.spec, utm.tape)).toEqual(tm);
    });
  });

  describe('recursion', () => {
    it('can simulate itself', () => {
      const doublySimulatedTm = makeInitSnapshot(checkPalindromeSpec, ["a", "b", "a"]);
      const simulatedUtm = makeInitSnapshot(myUtmSpec, myUtmSpec.encode(doublySimulatedTm));
      const utm = makeInitSnapshot(myUtmSpec, myUtmSpec.encode(simulatedUtm));

      expect(run(doublySimulatedTm)).toBe("accept");
      expect(doublySimulatedTm.tape).toEqual(["a", "b", "a"]);

      expect(run(simulatedUtm)).toBe("accept");
      expect(myUtmSpec.decode(doublySimulatedTm.spec, simulatedUtm.tape)).toEqual(doublySimulatedTm);

      expect(run(utm)).toBe("accept");
      expect(myUtmSpec.decode(myUtmSpec, utm.tape)).toEqual(simulatedUtm);
    });
  });
});
