// ════════════════════════════════════════════════════════════════════
// Infinite UTM tape: encode a UTM simulating itself
// ════════════════════════════════════════════════════════════════════

use std::{collections::HashMap, sync::LazyLock};

use crate::{
    tm::{RunningTuringMachine, TapeExtender, TuringMachineSpec},
    utm::{encode_tape, MyUtmEncodingScheme, Symbol, UtmEncodingScheme as _, UTM_SPEC},
};

static GUEST_SYM_TO_IDX: LazyLock<HashMap<Symbol, usize>> = LazyLock::new(|| {
    let guest_symbols = UTM_SPEC.iter_symbols().collect::<Vec<_>>();
    guest_symbols
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, i))
        .collect::<HashMap<Symbol, usize>>()
});

static HEADER: LazyLock<Vec<Symbol>> = LazyLock::new(|| {
    let full_tape = MyUtmEncodingScheme::encode(&RunningTuringMachine::new(&*UTM_SPEC));
    let caret_pos = full_tape
        .iter()
        .position(|&s| s == Symbol::Caret)
        .expect("encoded tape should contain ^");
    full_tape[..caret_pos].to_vec()
});

pub struct InfiniteTapeExtender;
impl TapeExtender<Symbol> for InfiniteTapeExtender {
    fn extend(&self, tape: &mut Vec<Symbol>, min_size: usize) {
        let (mut tape_start_index, mut chunk) = (0, HEADER.clone());
        while tape_start_index + chunk.len() < min_size {
            tape_start_index += chunk.len();
            chunk = encode_tape(&GUEST_SYM_TO_IDX, tape)
        }

        tape.extend_from_slice(&chunk.as_slice()[tape.len() - tape_start_index..]);
    }
}
