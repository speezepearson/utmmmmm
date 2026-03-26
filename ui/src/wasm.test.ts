import { describe, expect, it } from "vitest";
import {
  type TuringMachineSnapshot,
  type Symbol,
  makeInitSnapshot,
  getStatus,
  step,
} from "./types";
import { machineSpecs, type ParsedSpec } from "./parseSpec";
import rawSpecs from "./machine-specs.json";

// Import WASM encode/decode from the Node.js-targeted build
// (the web-targeted build in ../pkg is for the browser UI)
import {
  encode as wasmEncode,
  decode as wasmDecode,
} from "../pkg-node/utmmmmm.js";

// ── helpers ──

function getSpec(name: string): ParsedSpec {
  const s = machineSpecs.find((s) => s.name === name);
  if (!s) throw new Error(`Spec "${name}" not found`);
  return s;
}

function getRawJson(name: string): string {
  const raw = (rawSpecs as Array<Record<string, unknown>>).find(
    (s) => s.name === name,
  );
  if (!raw) throw new Error(`Raw spec "${name}" not found`);
  return JSON.stringify(raw);
}

/** Run a TM to completion (synchronously, no yielding). */
function runSync(
  snapshot: TuringMachineSnapshot,
  maxSteps: number,
): TuringMachineSnapshot {
  for (let i = 0; i < maxSteps; i++) {
    if (getStatus(snapshot) !== "running") return snapshot;
    step(snapshot);
  }
  return snapshot;
}

/** Strip trailing blanks from a tape. */
function stripTrailingBlanks(tape: Symbol[], blank: Symbol): Symbol[] {
  let end = tape.length;
  while (end > 1 && tape[end - 1] === blank) end--;
  return tape.slice(0, end);
}

// ── tests ──

describe("flip bits via UTM encode/decode round-trip", () => {
  it("running flip bits directly on 01011 produces 10100", () => {
    const flipBits = getSpec("Flip Bits");
    const tape = "01011".split("") as Symbol[];
    const snapshot = makeInitSnapshot(flipBits.spec, tape);
    runSync(snapshot, 100);

    expect(getStatus(snapshot)).toBe("accept");
    const result = stripTrailingBlanks(snapshot.tape, flipBits.spec.blank);
    expect(result.join("")).toBe("10100");
  });

  it("encode+decode round-trip preserves initial state", () => {
    const flipBitsJson = getRawJson("Flip Bits");
    const encoded: string = wasmEncode(flipBitsJson, "01011");

    // Verify the encoded tape starts with $ and contains expected structure
    expect(encoded[0]).toBe("$");
    expect(encoded).toContain("#");

    // Decode it back
    const decodedJson: string = wasmDecode(flipBitsJson, encoded);
    const decoded = JSON.parse(decodedJson) as {
      state: string;
      tape: string;
      pos: number;
    };
    expect(decoded.state).toBe("Flip"); // initial state
    expect(decoded.tape).toBe("01011"); // original tape
    expect(decoded.pos).toBe(0); // head at start
  });

  it("encode+(run on UTM)+decode gives same result as direct execution", () => {
      const flipBits = getSpec("Flip Bits");
      const flipBitsJson = getRawJson("Flip Bits");
      const utm = getSpec("Universal Turing Machine");

      // 1. Run flip bits directly on "01011"
      const directTape = "01011".split("") as Symbol[];
      const directSnapshot = makeInitSnapshot(flipBits.spec, directTape);
      runSync(directSnapshot, 100);
      expect(getStatus(directSnapshot)).toBe("accept");
      const directResult = stripTrailingBlanks(
        directSnapshot.tape,
        flipBits.spec.blank,
      );

      // 2. Encode the flip bits machine + tape into UTM tape via Rust WASM
      const encodedTapeStr: string = wasmEncode(flipBitsJson, "01011");
      const utmTape = encodedTapeStr.split("") as Symbol[];

      // 3. Run the UTM on the encoded tape (in TypeScript)
      const utmSnapshot = makeInitSnapshot(utm.spec, utmTape);
      runSync(utmSnapshot, 15_000_000);
      const utmStatus = getStatus(utmSnapshot);
      expect(utmStatus).toBe("accept");

      // 4. Decode the UTM tape back to guest state via Rust WASM
      const utmTapeStr = utmSnapshot.tape.join("");
      const decodedJson: string = wasmDecode(flipBitsJson, utmTapeStr);
      const decoded = JSON.parse(decodedJson) as {
        state: string;
        tape: string;
        pos: number;
      };

      // 5. Verify: decoded tape matches direct execution
      const decodedResult = stripTrailingBlanks(
        decoded.tape.split("") as Symbol[],
        flipBits.spec.blank,
      );
      expect(decoded.state).toBe("Done"); // accepting state
      expect(decodedResult.join("")).toBe(directResult.join(""));
      expect(decodedResult.join("")).toBe("10100");
    });

});
