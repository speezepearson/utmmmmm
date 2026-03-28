use crate::tm::{RunningTuringMachine, TuringMachineSpec};

pub trait UtmSpec: TuringMachineSpec {
    fn encode<Guest: TuringMachineSpec>(
        &self,
        tm: &RunningTuringMachine<Guest>,
    ) -> Vec<Self::Symbol>;
    fn decode<'a, Guest: TuringMachineSpec>(
        &self,
        guest: &'a Guest,
        tape: &[Self::Symbol],
    ) -> Result<RunningTuringMachine<'a, Guest>, String>;

    /// Returns true once per completed inner step. Decoding the tape at a
    /// tick should yield a valid snapshot of the guest machine after one
    /// more step than at the previous tick.
    fn at_tick(&self, state: Self::State, symbol: Self::Symbol) -> bool;
}
