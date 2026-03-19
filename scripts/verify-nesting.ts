import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { flipBitsSpec } from "../src/toy-machines";
import {
  copySnapshot,
  getStatus,
  makeInitSnapshot,
  step,
} from "../src/types";
import { MyUtmSnapshot, type MyUtmState, type MyUtmSymbol } from "../src/my-utm-spec";
import { tmsEqual } from "../src/util";

const SAVEPOINT_FILE = "verify-nesting.savepoint.json";

function padN(n: number, width: number) {
  return n.toString().padStart(width, " ");
}

type Savepoint = {
  steps: number;
  innerSteps: number;
  simulator: {
    state: MyUtmState;
    tape: MyUtmSymbol[];
    pos: number;
  };
};

let simulator: MyUtmSnapshot<typeof flipBitsSpec['allStates'][number], typeof flipBitsSpec['allSymbols'][number]>;
let doubleSimulator: MyUtmSnapshot<MyUtmState, MyUtmSymbol>;
let steps: number;
let innerSteps: number;

const loadArg = process.argv.includes("--load");
if (loadArg && existsSync(SAVEPOINT_FILE)) {
  const data: Savepoint = JSON.parse(readFileSync(SAVEPOINT_FILE, "utf-8"));
  simulator = new MyUtmSnapshot({...data.simulator, simSpec: flipBitsSpec});
  doubleSimulator = MyUtmSnapshot.fromSimSnapshot(simulator);
  steps = data.steps;
  innerSteps = data.innerSteps;
  console.log(
    `Loaded savepoint: steps=${steps}, innerSteps=${innerSteps}`,
  );
} else {
  simulator = MyUtmSnapshot.fromSimSnapshot(makeInitSnapshot(flipBitsSpec, ["0"]));
  doubleSimulator = MyUtmSnapshot.fromSimSnapshot(simulator);
  steps = 0;
  innerSteps = 0;
}

const expectedInnerSteps = (() => {
  const utm = copySnapshot(simulator);
  let steps = 0;
  while (getStatus(step(utm)) === "running") {
    steps++;
  }
  return steps;
})();

let lastInnerTickT = Date.now();
while (true) {
  if (getStatus(step(doubleSimulator)) !== "running") {
    break;
  }
  const decoded = doubleSimulator.decode();
  if (decoded && decoded.pos !== simulator.pos) {
    innerSteps++;
    step(simulator);
    if (!tmsEqual(simulator, decoded)) {
      console.error("simulator and decoded are not equal!");
      console.error(`  simulator.state = ${simulator.state}`);
      console.error(`  decoded.state   = ${decoded.state}`);
      console.error(`  simulator.pos   = ${simulator.pos}`);
      console.error(`  decoded.pos     = ${decoded.pos}`);
      console.error(`  simulator.tape.length = ${simulator.tape.length}`);
      console.error(`  decoded.tape.length   = ${decoded.tape.length}`);
      // Find first differing tape cell
      const maxLen = Math.max(simulator.tape.length, decoded.tape.length);
      for (let i = 0; i < maxLen; i++) {
        const s = i < simulator.tape.length ? simulator.tape[i] : "<missing>";
        const d = i < decoded.tape.length ? decoded.tape[i] : "<missing>";
        if (s !== d) {
          console.error(
            `  first tape diff at index ${i}: simulator=${s}, decoded=${d}`,
          );
          break;
        }
      }
      throw new Error("simulator and decoded are not equal");
    }
    const now = Date.now();
    console.log(
      `[step=${padN(steps, 11)}] ticked the simulated machine! (dur=${padN(Math.round(now - lastInnerTickT), 4)}ms)`,
    );
    console.log(
      `${padN(innerSteps, 6)}/${padN(expectedInnerSteps, 6)} : ${decoded.tape.join("")} => ${new MyUtmSnapshot({...decoded, simSpec: flipBitsSpec}).tape.join("")}`,
    );
    console.log(
      Array(6 + 1 + 6 + 3 + decoded.pos)
        .fill(" ")
        .join("") + `^ (state=${decoded.state})`,
    );
    lastInnerTickT = now;
  }
  steps++;
  if (steps > 1e11) {
    throw new Error("doesn't look like we're getting anywhere");
  }
  if (steps % 1e7 === 0) {
    const savepoint: Savepoint = {
      steps,
      innerSteps,
      simulator,
    };
    writeFileSync(SAVEPOINT_FILE, JSON.stringify(savepoint));
  }
}

console.log(`[step=${padN(steps, 11)}] done`);
