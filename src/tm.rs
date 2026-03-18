use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct TuringMachine<S, A> {
    pub initial: S,
    pub accept: S,
    pub reject: S,
    pub blank: A,
    pub transitions: HashMap<(S, A), (S, A, Dir)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult<A> {
    pub outcome: Outcome,
    /// Tape contents from the leftmost visited cell to the rightmost visited cell.
    pub tape: Vec<A>,
}

/// Mutable in-place TM execution state.
pub struct TmState<S, A> {
    pub state: S,
    pub tape: HashMap<i64, A>,
    pub head: i64,
    pub min_pos: i64,
    pub max_pos: i64,
    pub halted: bool,
    pub outcome: Option<Outcome>,
    pub steps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Continue,
    Halted,
}

impl<S, A> TmState<S, A>
where
    S: Eq + Hash + Clone + Debug,
    A: Eq + Hash + Clone + Debug,
{
    pub fn new(tm: &TuringMachine<S, A>, input: &[A]) -> Self {
        let mut tape: HashMap<i64, A> = HashMap::new();
        for (i, sym) in input.iter().enumerate() {
            tape.insert(i as i64, sym.clone());
        }
        TmState {
            state: tm.initial.clone(),
            tape,
            head: 0,
            min_pos: 0,
            max_pos: input.len().saturating_sub(1) as i64,
            halted: false,
            outcome: None,
            steps: 0,
        }
    }

    pub fn step(&mut self, tm: &TuringMachine<S, A>) -> StepResult {
        if self.halted {
            return StepResult::Halted;
        }

        if self.state == tm.accept {
            self.halted = true;
            self.outcome = Some(Outcome::Accept);
            return StepResult::Halted;
        }
        if self.state == tm.reject {
            self.halted = true;
            self.outcome = Some(Outcome::Reject);
            return StepResult::Halted;
        }

        let sym = self.tape.get(&self.head).cloned().unwrap_or_else(|| tm.blank.clone());
        let (new_state, new_sym, dir) = tm
            .transitions
            .get(&(self.state.clone(), sym))
            .unwrap_or_else(|| {
                panic!(
                    "No transition for ({:?}, {:?})",
                    self.state,
                    self.tape.get(&self.head).cloned().unwrap_or_else(|| tm.blank.clone())
                )
            })
            .clone();

        self.tape.insert(self.head, new_sym);
        self.state = new_state;

        match dir {
            Dir::Left => self.head -= 1,
            Dir::Right => self.head += 1,
        }

        self.min_pos = self.min_pos.min(self.head);
        self.max_pos = self.max_pos.max(self.head);
        self.steps += 1;

        StepResult::Continue
    }

    pub fn read(&self, pos: i64, blank: &A) -> A {
        self.tape.get(&pos).cloned().unwrap_or_else(|| blank.clone())
    }

    pub fn collect_tape(&self, blank: &A) -> Vec<A> {
        collect_tape(&self.tape, blank, self.min_pos, self.max_pos)
    }
}

/// Run a TM to completion (or until `max_steps` is exceeded).
/// Returns `None` if the machine doesn't halt within `max_steps`.
pub fn run<S, A>(tm: &TuringMachine<S, A>, input: &[A], max_steps: usize) -> Option<RunResult<A>>
where
    S: Eq + Hash + Clone + Debug,
    A: Eq + Hash + Clone + Debug,
{
    // Tape stored as a HashMap from position to symbol.
    let mut tape: HashMap<i64, A> = HashMap::new();
    for (i, sym) in input.iter().enumerate() {
        tape.insert(i as i64, sym.clone());
    }

    let mut head: i64 = 0;
    let mut state = tm.initial.clone();
    let mut min_pos: i64 = 0;
    let mut max_pos: i64 = input.len().saturating_sub(1) as i64;

    for _ in 0..max_steps {
        if state == tm.accept {
            return Some(RunResult {
                outcome: Outcome::Accept,
                tape: collect_tape(&tape, &tm.blank, min_pos, max_pos),
            });
        }
        if state == tm.reject {
            return Some(RunResult {
                outcome: Outcome::Reject,
                tape: collect_tape(&tape, &tm.blank, min_pos, max_pos),
            });
        }

        let sym = tape.get(&head).cloned().unwrap_or_else(|| tm.blank.clone());
        let (new_state, new_sym, dir) = tm
            .transitions
            .get(&(state.clone(), sym))
            .unwrap_or_else(|| panic!("No transition for ({:?}, {:?})", state, tape.get(&head).cloned().unwrap_or_else(|| tm.blank.clone())))
            .clone();

        tape.insert(head, new_sym);
        state = new_state;

        match dir {
            Dir::Left => head -= 1,
            Dir::Right => head += 1,
        }

        min_pos = min_pos.min(head);
        max_pos = max_pos.max(head);
    }

    None // didn't halt
}

fn collect_tape<A: Clone>(
    tape: &HashMap<i64, A>,
    blank: &A,
    min_pos: i64,
    max_pos: i64,
) -> Vec<A> {
    (min_pos..=max_pos)
        .map(|i| tape.get(&i).cloned().unwrap_or_else(|| blank.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial TM that scans right, flipping 0->1 and 1->0, until it hits blank, then accepts.
    /// States: Flip, Accept, Reject
    /// Alphabet: 0, 1, Blank
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum FlipState {
        Flip,
        Accept,
        Reject,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum FlipSym {
        Zero,
        One,
        Blank,
    }

    fn flip_tm() -> TuringMachine<FlipState, FlipSym> {
        let mut transitions = HashMap::new();
        // Flip: 0 -> write 1, move right, stay in Flip
        transitions.insert(
            (FlipState::Flip, FlipSym::Zero),
            (FlipState::Flip, FlipSym::One, Dir::Right),
        );
        // Flip: 1 -> write 0, move right, stay in Flip
        transitions.insert(
            (FlipState::Flip, FlipSym::One),
            (FlipState::Flip, FlipSym::Zero, Dir::Right),
        );
        // Flip: Blank -> accept
        transitions.insert(
            (FlipState::Flip, FlipSym::Blank),
            (FlipState::Accept, FlipSym::Blank, Dir::Left),
        );

        TuringMachine {
            initial: FlipState::Flip,
            accept: FlipState::Accept,
            reject: FlipState::Reject,
            blank: FlipSym::Blank,
            transitions,
        }
    }

    #[test]
    fn test_flip_empty() {
        let tm = flip_tm();
        let result = run(&tm, &[], 100).unwrap();
        assert_eq!(result.outcome, Outcome::Accept);
    }

    #[test]
    fn test_flip_bits() {
        let tm = flip_tm();
        let input = vec![FlipSym::Zero, FlipSym::One, FlipSym::Zero, FlipSym::One];
        let result = run(&tm, &input, 100).unwrap();
        assert_eq!(result.outcome, Outcome::Accept);
        // Tape should have flipped bits, plus the blank cell the head visited
        assert_eq!(
            result.tape,
            vec![
                FlipSym::One,
                FlipSym::Zero,
                FlipSym::One,
                FlipSym::Zero,
                FlipSym::Blank
            ]
        );
    }

    #[test]
    fn test_flip_all_zeros() {
        let tm = flip_tm();
        let input = vec![FlipSym::Zero, FlipSym::Zero, FlipSym::Zero];
        let result = run(&tm, &input, 100).unwrap();
        assert_eq!(result.outcome, Outcome::Accept);
        assert_eq!(
            result.tape,
            vec![FlipSym::One, FlipSym::One, FlipSym::One, FlipSym::Blank]
        );
    }

    /// A TM that accepts strings of the form 0^n 1^n.
    /// Cross off matching 0s and 1s by replacing them with X.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum MatchState {
        ScanRight,   // looking for a 0 to cross off
        FindOne,     // found a 0 (replaced with X), scanning right past 0s and Xs to find a 1
        GoBack,      // found and crossed off a 1, heading back to start
        Accept,
        Reject,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum MatchSym {
        Zero,
        One,
        X,
        Blank,
    }

    fn match_tm() -> TuringMachine<MatchState, MatchSym> {
        use MatchState::*;
        use MatchSym::*;
        let mut t = HashMap::new();

        // ScanRight: skip Xs, find 0 or blank
        t.insert((ScanRight, X), (ScanRight, X, Dir::Right));
        t.insert((ScanRight, Zero), (FindOne, X, Dir::Right));    // cross off 0
        t.insert((ScanRight, Blank), (Accept, Blank, Dir::Right)); // all matched
        t.insert((ScanRight, One), (Reject, One, Dir::Right));     // unmatched 1

        // FindOne: skip 0s and Xs to find matching 1
        t.insert((FindOne, Zero), (FindOne, Zero, Dir::Right));
        t.insert((FindOne, X), (FindOne, X, Dir::Right));
        t.insert((FindOne, One), (GoBack, X, Dir::Left));         // cross off 1
        t.insert((FindOne, Blank), (Reject, Blank, Dir::Right));  // no matching 1

        // GoBack: go all the way left to start
        t.insert((GoBack, Zero), (GoBack, Zero, Dir::Left));
        t.insert((GoBack, X), (GoBack, X, Dir::Left));
        t.insert((GoBack, One), (GoBack, One, Dir::Left));
        t.insert((GoBack, Blank), (ScanRight, Blank, Dir::Right));

        TuringMachine {
            initial: ScanRight,
            accept: Accept,
            reject: Reject,
            blank: Blank,
            transitions: t,
        }
    }

    #[test]
    fn test_match_accepts() {
        let tm = match_tm();
        for input in [
            vec![],
            vec![MatchSym::Zero, MatchSym::One],
            vec![
                MatchSym::Zero,
                MatchSym::Zero,
                MatchSym::One,
                MatchSym::One,
            ],
        ] {
            let result = run(&tm, &input, 1000).unwrap();
            assert_eq!(result.outcome, Outcome::Accept, "should accept {:?}", input);
        }
    }

    #[test]
    fn test_match_rejects() {
        let tm = match_tm();
        for input in [
            vec![MatchSym::One],
            vec![MatchSym::Zero],
            vec![MatchSym::Zero, MatchSym::Zero, MatchSym::One],
            vec![MatchSym::Zero, MatchSym::One, MatchSym::One],
        ] {
            let result = run(&tm, &input, 1000).unwrap();
            assert_eq!(result.outcome, Outcome::Reject, "should reject {:?}", input);
        }
    }
}
