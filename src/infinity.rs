// ════════════════════════════════════════════════════════════════════
// Infinite UTM tape: encode a UTM simulating itself (fixed point)
//
// The tape is: HEADER ^cell0 ,cell1 ,cell2 ...
// where cell_k encodes tape[k]. Since the guest is the UTM itself,
// tape[k] for k < |HEADER| is the k-th header symbol, and for
// k >= |HEADER| it's a tape-section symbol (marker or bit).
// Each cell_k only depends on tape[k] which is already computed
// (since k < HEADER_LEN + k * CELL_WIDTH), so this is a well-defined
// fixed point we can compute left-to-right.
// ════════════════════════════════════════════════════════════════════

use std::{collections::HashMap, sync::LazyLock};

use crate::{
    compiled::{CSymbol, CompiledTuringMachineSpec},
    optimization_hints::OPTIMIZATION_HINTS,
    tm::{RunningTuringMachine, SimpleTuringMachineSpec, TuringMachineSpec},
    utm::{num_bits, MyUtmEncodingScheme, State, Symbol, UTM_SPEC},
};

/// The header: everything before the tape section ($ # rules # acc # state # blank #).
pub static HEADER: LazyLock<Vec<Symbol>> = LazyLock::new(|| {
    let dummy = MyUtmEncodingScheme::encode_with_rule_order(
        &RunningTuringMachine::new(&*UTM_SPEC),
        Some(OPTIMIZATION_HINTS),
    );
    let caret_pos = dummy
        .iter()
        .position(|&s| s == Symbol::Caret)
        .expect("encoded tape should contain ^");
    dummy[..caret_pos].to_vec()
});

static GUEST_SYM_TO_IDX: LazyLock<HashMap<Symbol, usize>> = LazyLock::new(|| {
    UTM_SPEC
        .iter_symbols()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect()
});

static N_SYM_BITS: LazyLock<usize> = LazyLock::new(|| num_bits(UTM_SPEC.iter_symbols().count()));

/// The length of the UTM header (everything before the `^` in the encoded tape).
pub fn header_len() -> usize {
    HEADER.len()
}

pub struct InfiniteTape(Vec<Symbol>);

impl InfiniteTape {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn get_realized(&self) -> &[Symbol] {
        &self.0
    }

    pub fn get(&mut self, index: usize) -> Symbol {
        self.extend_self(index);
        self.0[index]
    }

    pub fn iter_forever<'a>(&'a mut self) -> impl Iterator<Item = Symbol> + 'a {
        (0..).map(|i| self.get(i))
    }

    pub fn extend(&mut self, dst: &mut Vec<Symbol>, index: usize) {
        if dst.len() >= index {
            return;
        }
        self.extend_self(index);
        dst.extend_from_slice(&self.0[dst.len()..index]);
    }

    pub fn extend_compiled(
        &mut self,
        dst: &mut Vec<CSymbol>,
        index: usize,
        spec: &CompiledTuringMachineSpec<SimpleTuringMachineSpec<State, Symbol>>,
    ) {
        if dst.len() >= index {
            return;
        }
        self.extend_self(index);
        dst.extend(
            self.0[dst.len()..index]
                .iter()
                .map(|&s| spec.compile_symbol(s)),
        );
    }

    fn extend_self(&mut self, index: usize) {
        if index < self.0.len() {
            return;
        }

        let header = &*HEADER;
        let sym_to_idx = &*GUEST_SYM_TO_IDX;
        let n_sym_bits = *N_SYM_BITS;
        let cell_width = 1 + n_sym_bits; // marker + bits
        let header_len = header.len();

        while self.0.len() <= index {
            let pos = self.0.len();
            if pos < header_len {
                self.0.push(header[pos]);
            } else {
                let offset = pos - header_len;
                if offset % cell_width == 0 {
                    // Marker position
                    let cell_index = offset / cell_width;
                    self.0.push(if cell_index == 0 {
                        Symbol::Caret
                    } else {
                        Symbol::Comma
                    });
                } else {
                    // Bit position: encode self.0[cell_index]
                    let cell_index = offset / cell_width;
                    let bit_offset = offset % cell_width - 1;

                    let sym = self.0[cell_index]; // always available: cell_index < pos
                    let sym_idx = sym_to_idx[&sym];
                    let bit = (sym_idx >> (n_sym_bits - 1 - bit_offset)) & 1;
                    self.0
                        .push(if bit == 1 { Symbol::One } else { Symbol::Zero });
                }
            }
        }
    }
}
