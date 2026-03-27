use crate::tm::{RunningTuringMachine, TuringMachineSpec};

pub trait UtmSpec: TuringMachineSpec {
    fn encode_tape<Guest: TuringMachineSpec>(
        &self,
        guest: &Guest,
        tape: &[Guest::Symbol],
    ) -> Vec<Self::Symbol>;
    fn encode<Guest: TuringMachineSpec>(
        &self,
        tm: &RunningTuringMachine<Guest>,
    ) -> Vec<Self::Symbol>;
    fn decode<'a, Guest: TuringMachineSpec>(
        &self,
        guest: &'a Guest,
        tape: &[Self::Symbol],
    ) -> Result<RunningTuringMachine<'a, Guest>, String>;
}
