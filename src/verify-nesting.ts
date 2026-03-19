import { isDeepStrictEqual } from "node:util";
import { makeInitUtmSnapshot, myUtmSpec } from "./my-utm-spec";
import { flipBitsSpec } from "./toy-machines";
import { getStatus, makeInitSnapshot, step } from "./types";

function padN(n: number, width: number) {
  return n.toString().padStart(width, ' ');
}

const expectedInnerSteps = (() => {
  const utm = makeInitUtmSnapshot(makeInitSnapshot(flipBitsSpec, ["0"]));
  let steps = 0;
  while (getStatus(step(utm)) === 'running') {
    steps++;
  }
  return steps;
})();

// TODO: load these states from temp.savepoint.json
const simulator = makeInitUtmSnapshot(makeInitSnapshot(flipBitsSpec, ["0"]));
const doubleSimulator = makeInitUtmSnapshot(simulator);

let lastInnerTickT = Date.now();
let steps = 0;
let innerSteps = 0;
while (true) {
  if (getStatus(step(doubleSimulator)) !== 'running') {
    break;
  }
  const decoded = myUtmSpec.decode(myUtmSpec, doubleSimulator);
  if (decoded && (decoded.pos !== simulator.pos)) {
    innerSteps++;
    step(simulator);
    if (!isDeepStrictEqual(simulator, decoded)) {
        throw new Error("simulator and decoded are not equal");
    }
    const now = Date.now();
    console.log(`[step=${padN(steps, 11)}] ticked the simulated machine! (dur=${padN(Math.round(now - lastInnerTickT), 4)}ms)`);
    console.log(`${padN(innerSteps, 6)}/${padN(expectedInnerSteps, 6)} : ${decoded.tape.join("")} => ${myUtmSpec.decode(flipBitsSpec, decoded)?.tape.join("")}`);
    console.log(Array(6+1+6+3+decoded.pos).fill(' ').join('') + `^ (state=${decoded.state})`);
    lastInnerTickT = now;
  }
  steps++;
  if (steps > 1e11) {
    throw new Error("doesn't look like we're getting anywhere")
  }
  if (steps % 1e7 === 0) {
    // TODO: persist the simulator and doubleSimulator states to temp.savepoint.json
  }
}

console.log(`[step=${padN(steps, 11)}] done`);
