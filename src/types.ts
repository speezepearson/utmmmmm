export type Dir = "L" | "R";

export type TuringMachineSpec<State extends string, Symbol extends string> = {
  readonly allStates: ReadonlyArray<State>;
  readonly allSymbols: ReadonlyArray<Symbol>;
  readonly initial: State;
  readonly blank: Symbol;
  readonly rules: Readonly<
    Record<
      State,
      Readonly<
        Record<
          Symbol,
          | { type: "accept" }
          | { type: "reject" }
          | { type: "step"; newState: State; newSymbol: Symbol; dir: Dir }
        >
      >
    >
  >;
};

export type TuringMachineSnapshot<
  State extends string,
  Symbol extends string,
> = {
  spec: TuringMachineSpec<State, Symbol>;
  state: State;
  tape: Symbol[];
  pos: number;
};

export function makeInitSnapshot<State extends string, Symbol extends string>(
  spec: TuringMachineSpec<State, Symbol>,
  tape: readonly Symbol[] = [],
): TuringMachineSnapshot<State, Symbol> {
  return {
    spec,
    state: spec.initial,
    tape: tape.slice(),
    pos: 0,
  };
}
export function copySnapshot<State extends string, Symbol extends string>(
  snapshot: TuringMachineSnapshot<State, Symbol>,
): TuringMachineSnapshot<State, Symbol> {
  return {
    spec: snapshot.spec,
    state: snapshot.state,
    tape: snapshot.tape.slice(),
    pos: snapshot.pos,
  };
}

export function step<State extends string, Symbol extends string>(
  snapshot: TuringMachineSnapshot<State, Symbol>,
): "accept" | "reject" | "continue" {
  const { spec, state, tape, pos } = snapshot;
  const symbol = pos < tape.length ? tape[pos] : spec.blank;
  const rule = spec.rules[state][symbol];
  switch (rule.type) {
    case "accept":
      return "accept";
    case "reject":
      return "reject";
    case "step": {
      snapshot.state = rule.newState;
      while (snapshot.tape.length <= pos) {
        snapshot.tape.push(spec.blank);
      }
      snapshot.tape[pos] = rule.newSymbol;
      if (rule.dir === "L" && pos === 0) {
        throw new Error("Can't step machine; already at left edge of tape");
      }
      snapshot.pos = { L: pos - 1, R: pos + 1 }[rule.dir];
      return "continue";
    }
  }
}
export function run<State extends string, Symbol extends string>(snapshot: TuringMachineSnapshot<State, Symbol>): "accept" | "reject" {
  while (true) {
    switch (step(snapshot)) {
      case "accept":
        return "accept";
      case "reject":
        return "reject";
      case "continue":
        // noop
        break;
    }
  }
}

export type UtmSpec<
  UState extends string,
  USymbol extends string,
> = TuringMachineSpec<UState, USymbol> & {
  encode<SimState extends string, SimSymbol extends string>(
    snapshot: TuringMachineSnapshot<SimState, SimSymbol>,
  ): USymbol[];

  /** Decodes the tape of a running UTM into a snapshot of the simulated machine. May return undefined if the UTM is mid-operation.
   * Should always yield the simulated TM's snapshots in-order: so if the simulated TM is in state X, then steps to Y, then steps to Z,
   * then, when we simulate it with a UTM, `decode()`ing the UTM's tape should return:
   * - X for a while...
   * - then optionally undefined for a while...
   * - then Y for a while...
   * - then optionally undefined for a while...
   * - then Z for a while...
   */
  decode<SimState extends string, SimSymbol extends string>(
    spec: TuringMachineSpec<SimState, SimSymbol>,
    uTape: readonly USymbol[],
  ): undefined | TuringMachineSnapshot<SimState, SimSymbol>;
};

export function assertNever(x: never): never {
  throw new Error(`Unexpected value: ${x}`);
}
