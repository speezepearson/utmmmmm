#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct State(u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Symbol(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TuringMachineSpec {
    initial: State,
    blank: Symbol, // not actually used in the infinite-tape case, but the UTM *should* be able to
    accept: State,
    transition_matrix: [Option<(State, Symbol, Dir)>; 1 << 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunningTuringMachine {
    spec: TuringMachineSpec,
    state: State,
    pos: usize,
    tape: Vec<Symbol>,
}

fn build_utm_spec() -> TuringMachineSpec {
    // equivalent to the TypeScript `myUtmSpec`
    todo!()
}

fn build_header(spec: TuringMachineSpec, optimization_hints: Vec<(State, Symbol)>) -> Vec<Symbol> {
    // equivalent to the TypeScript `buildHeader(myUtmSpec, myUtmSpec.initial, optimizationHints)`
    todo!()
}

fn extend_infinite_utm_tape(tape: &mut Vec<Symbol>) {
    let header = build_header(
        build_utm_spec(),
        todo!("see TypeScript infiniteUtmTapeBackground"),
    ); // ideally we would just compute this once, make it static or something

    // Equivalent to the TypeScript `infiniteUtmTapeBackground`.
    // Increase the tape's length by some substantial amount.
    // Do not overwrite any existing values.
    todo!()
}

fn step_turing_machine(
    m: &mut RunningTuringMachine,
    extend_tape: impl Fn(&mut Vec<Symbol>, Symbol),
) -> bool {
    while m.pos >= m.tape.len() {
        extend_tape(&mut m.tape, m.spec.blank);
    }
    let symbol = m.tape[m.pos];
    if let Some((next_state, next_symbol, dir)) =
        m.spec.transition_matrix[((m.state.0 as usize) << 8) | (symbol.0 as usize)]
    {
        m.state = next_state;
        m.tape[m.pos] = next_symbol;
        m.pos = match dir {
            Dir::Left => m.pos.saturating_sub(1),
            Dir::Right => m.pos + 1,
        };
        true
    } else {
        false
    }
}

fn main() {
    let spec = build_utm_spec();
    let mut machine = RunningTuringMachine {
        spec: spec.clone(),
        state: spec.initial,
        pos: 0,
        tape: vec![],
    };
    let mut steps = 0;
    let extend_tape = |tape: &mut Vec<Symbol>, _blank: Symbol| {
        extend_infinite_utm_tape(tape);
    };
    loop {
        steps += 1;
        if !step_turing_machine(&mut machine, extend_tape) {
            break;
        }
    }
    println!(
        "halted in state {:?} ({}) after {steps} steps",
        machine.state,
        if machine.state == spec.accept {
            "accept"
        } else {
            "reject"
        }
    );
}
