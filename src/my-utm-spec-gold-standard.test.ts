/**
 * DO NOT MODIFY THIS FILE.
 */

import { describe, expect, it } from "vitest";
import {
  getStatus,
  makeInitSnapshot,
  run,
  step,
  type TuringMachineSnapshot,
} from "./types";
import { myUtmSpec } from "./my-utm-spec";
import {
  acceptImmediatelySpec,
  checkPalindromeSpec,
  doubleXSpec,
  flipBitsSpec,
  rejectImmediatelySpec,
} from "./toy-machines";
import { tmsEqual } from "./util";
import { expectTmsEqual, must } from "./test-util";

function listAllSnapshots<TM extends TuringMachineSnapshot<string, string>>(
  tm: TM,
): Array<TM> {
  const res = [];
  while (getStatus(tm) === "running") {
    res.push(structuredClone(tm));
    step(tm);
  }
  res.push(tm);
  return res;
}

const variousSnapshots = [
  makeInitSnapshot(acceptImmediatelySpec, []),
  makeInitSnapshot(rejectImmediatelySpec, []),
  ...listAllSnapshots(makeInitSnapshot(flipBitsSpec, ["0", "1"])),
  ...listAllSnapshots(makeInitSnapshot(doubleXSpec, ["$", "X", "X"])),
  ...listAllSnapshots(makeInitSnapshot(checkPalindromeSpec, ["a", "a"])),
  ...listAllSnapshots(makeInitSnapshot(checkPalindromeSpec, ["b", "b"])),
  ...listAllSnapshots(makeInitSnapshot(checkPalindromeSpec, ["a", "b"])),
];

describe("myUtmSpec gold standard tests", () => {
  describe("decode", () => {
    it.each(variousSnapshots)("inverts encode", (tm) => {
      const encoded = myUtmSpec.encode(tm);
      expectTmsEqual(must(encoded.decode()), tm);
    });

    it("can encode/decode itself", () => {
      const simulated = myUtmSpec.encode(makeInitSnapshot(flipBitsSpec, ["0"]));
      const simulator = myUtmSpec.encode(simulated);
      const decoded = simulator.decode();
      expectTmsEqual(must(decoded), simulated);
    });
  });

  describe("rules", { timeout: 20_000 }, () => {
    it.each(variousSnapshots)(
      "decodes to (original snapshot / undefined) for a while, then stepped snapshot",
      (tm) => {
        const utm = myUtmSpec.encode(tm);

        let snap = utm.decode();

        while (snap === undefined || tmsEqual(snap, tm)) {
          if (getStatus(utm) !== "running") break;
          step(utm);
          snap = utm.decode();
        }

        const snap1 = utm.decode();
        expectTmsEqual(must(snap1), step(tm));
      },
    );

    it("terminates with the same status as the simulated machine", () => {
      expect(getStatus(run(makeInitSnapshot(acceptImmediatelySpec, [])))).toBe(
        "accept",
      );
      expect(getStatus(run(makeInitSnapshot(rejectImmediatelySpec, [])))).toBe(
        "reject",
      );
    });

    it("terminates with the correct decoded tape", () => {
      const tm = makeInitSnapshot(flipBitsSpec, ["0", "1", "0", "1", "1"]);
      const utm = run(myUtmSpec.encode(tm));

      run(tm);
      run(utm);

      const decoded = utm.decode();
      expectTmsEqual(must(decoded), tm);
    });
  });

  describe("recursion", () => {
    it("can simulate itself", { timeout: 600_000 }, () => {
      const simulator = myUtmSpec.encode(
        myUtmSpec.encode(
          myUtmSpec.encode(makeInitSnapshot(flipBitsSpec, ["0"])),
        ),
      );
      const doubleSimulator = myUtmSpec.encode(simulator);

      let decoded;
      for (let i = 0; i < 1e9; i++) {
        expect(getStatus(step(doubleSimulator))).toBe("running");
        decoded = doubleSimulator.decode();
        if (decoded && decoded.pos !== simulator.pos) {
          break;
        }
      }
      const target = step(simulator);
      expectTmsEqual(must(decoded), target);
    });
  });
});
