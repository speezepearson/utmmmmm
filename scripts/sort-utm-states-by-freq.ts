import { myUtmSpec, type MyUtmState } from "../src/my-utm-spec";
import { flipBitsSpec } from "../src/toy-machines";
import { getStatus, makeInitSnapshot, step, type UtmSpec } from "../src/types";

type Stats<State extends string> = {
  steps: number;
  innerSteps: number;
  stateCounts: Partial<Record<State, number>>;
  statesSortedByFreqAsc: State[];
  freqStats: Partial<
    Record<
      State,
      {
        freq: number;
        cumFreq: number;
      }
    >
  >;
};

function getStats<State extends string, Symbol extends string>(
  utmSpec: UtmSpec<State, Symbol>,
  maxInnerSteps: number,
): Stats<State> {
  let base;
  {
    base = makeInitSnapshot(flipBitsSpec, ["0", "1"]);
    base = myUtmSpec.encode(base);
    base = utmSpec.encode(base);
  }
  const simulator = utmSpec.encode(base);
  const doubleSimulator = utmSpec.encode(simulator);
  let steps = 0;
  let innerSteps = 0;
  const stateCounts: Partial<Record<State, number>> = {};

  while (true) {
    if (getStatus(step(doubleSimulator)) !== "running") {
      break;
    }
    stateCounts[doubleSimulator.state] =
      (stateCounts[doubleSimulator.state] || 0) + 1;

    const decoded = doubleSimulator.decode();
    if (decoded && decoded.pos !== simulator.pos) {
      innerSteps++;
      console.error(`tick ${innerSteps}/${maxInnerSteps}`);
      if (innerSteps === maxInnerSteps) break;
    }
    steps++;
  }

  console.log(`took ${steps} steps`);
  const freqs = {} as Record<State, number>;
  for (const state of utmSpec.allStates) {
    freqs[state] = (stateCounts[state] ?? 0) / steps;
  }

  const statesSortedByFreqAsc = utmSpec.allStates.slice().sort((a, b) => {
    const freqA = freqs[a] ?? 0;
    const freqB = freqs[b] ?? 0;
    if (freqA !== freqB) return freqA - freqB;
    return a.localeCompare(b);
  });
  const freqStats = [];
  let lastCumFreq = 0;
  for (const state of statesSortedByFreqAsc) {
    freqStats.push({
      state,
      freq: freqs[state],
      cumFreq: lastCumFreq + freqs[state],
    });
    lastCumFreq += freqs[state];
  }

  const freqStatsObj: Partial<
    Record<State, { freq: number; cumFreq: number }>
  > = {};
  for (const { state, freq, cumFreq } of freqStats) {
    freqStatsObj[state as State] = { freq, cumFreq };
  }

  return {
    steps,
    innerSteps,
    stateCounts,
    statesSortedByFreqAsc,
    freqStats: freqStatsObj,
  };
}

function percentageDiff(a: number, b: number) {
  return (Math.abs(a - b) / Math.max(a, b)) * 100;
}

let spec = myUtmSpec;
let lastStats: Stats<MyUtmState> | undefined;
let nInnerSteps = 4;
while (true) {
  const t0 = performance.now();
  const stats = getStats(spec, nInnerSteps);
  const t1 = performance.now();
  console.log(stats);
  console.log(`took ${t1 - t0}ms`);
  console.log("recommendation:");
  console.log(JSON.stringify(stats.statesSortedByFreqAsc));
  if (lastStats) {
    const lastStatsConst = lastStats;
    const lastFreq = (st: MyUtmState) =>
      lastStatsConst.freqStats[st]?.freq ?? 0;
    const newFreq = (st: MyUtmState) => stats.freqStats[st]?.freq ?? 0;
    const changed = spec.allStates.filter(
      (st) =>
        percentageDiff(lastFreq(st), newFreq(st)) > 1 &&
        newFreq(st) > 1 / (10 * spec.allStates.length),
    );
    console.log(`${changed.length} states changed:`);
    for (const st of changed) {
      console.log(
        `  ${st.padEnd(20)} ${lastFreq(st).toFixed(6)} -> ${newFreq(st).toFixed(6)} (${percentageDiff(lastFreq(st), newFreq(st)).toFixed(2)}%)`,
      );
    }
    if (changed.length === 0) {
      console.log("reached fixed point, or close enough");
      break;
    }
    spec = {
      ...spec,
      allStates: spec.allStates.filter((st) => !changed.includes(st)),
    };
  }
  lastStats = stats;
  spec = { ...spec, allStates: stats.statesSortedByFreqAsc };
  nInnerSteps *= 2;
}
