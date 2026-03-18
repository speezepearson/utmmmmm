import {
  type TuringMachineSpec,
  type TuringMachineSnapshot,
  makeInitSnapshot,
  step,
} from "./types";

function printSnapshot<State extends string, Symbol extends string>(
  snapshot: TuringMachineSnapshot<State, Symbol>,
  stepNumber: number,
) {
  const tape =
    snapshot.tape.length > 0 ? snapshot.tape.join("") : snapshot.spec.blank;
  const pointer = " ".repeat(snapshot.pos) + `^ (state=${snapshot.state})`;
  console.log(`--- step ${stepNumber} ---`);
  console.log(tape);
  console.log(pointer);
}

export function trace<State extends string, Symbol extends string>(
  spec: TuringMachineSpec<State, Symbol>,
  initialTape: readonly Symbol[],
  maxSteps = 200,
) {
  const snapshot = makeInitSnapshot(spec, initialTape);
  // Pad tape so head always has a valid cell
  while (snapshot.tape.length <= snapshot.pos) {
    snapshot.tape.push(spec.blank);
  }

  printSnapshot(snapshot, 0);

  for (let i = 1; i <= maxSteps; i++) {
    while (snapshot.tape.length <= snapshot.pos) {
      snapshot.tape.push(spec.blank);
    }
    const result = step(snapshot);
    printSnapshot(snapshot, i);
    if (result !== "continue") {
      console.log(`\n=> ${result.toUpperCase()}`);
      return result;
    }
  }

  console.log(`\n=> (stopped after ${maxSteps} steps)`);
  return "timeout";
}

// CLI entry point
if (process.argv[1]?.endsWith("trace.ts")) {
  const { checkPalindromeSpec, write1sForeverSpec } =
    await import("./toy-machines");

  const machine = process.argv[2] ?? "palindrome";
  const input = process.argv[3] ?? "abba";

  if (machine === "palindrome") {
    const tape = input.split("") as ("a" | "b")[];
    console.log(`Palindrome checker on "${input}":\n`);
    trace(checkPalindromeSpec, tape);
  } else if (machine === "write1s") {
    console.log(`Write 1s forever:\n`);
    trace(write1sForeverSpec, [], 20);
  } else {
    console.error(`Unknown machine: ${machine}`);
    process.exit(1);
  }
}
