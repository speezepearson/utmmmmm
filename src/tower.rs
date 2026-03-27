use crate::compiled::{CState, CompiledTuringMachineSpec};
use crate::tm::{step, RunningTMStatus, RunningTuringMachine, SimpleTuringMachineSpec};
use crate::utm::UTM_SPEC;
use crate::utm::{MyUtmEncodingScheme, State, Symbol, UtmEncodingScheme};
use std::cmp::max;

pub type UtmTm<'a> = RunningTuringMachine<'a, SimpleTuringMachineSpec<State, Symbol>>;
pub type CompiledUtmSpec<'a> =
    CompiledTuringMachineSpec<'a, SimpleTuringMachineSpec<State, Symbol>>;

#[derive(Clone)]
pub struct TowerLevel<TM> {
    pub tm: TM,
    pub total_steps: u64,
    pub max_head_pos: usize,
}
pub type CompiledTowerLevel<'a> = TowerLevel<RunningTuringMachine<'a, CompiledUtmSpec<'a>>>;
pub type UtmTowerLevel<'a> =
    TowerLevel<RunningTuringMachine<'a, SimpleTuringMachineSpec<State, Symbol>>>;

pub struct Tower<'a> {
    pub base: CompiledTowerLevel<'a>,
    pub decoded: Vec<UtmTowerLevel<'a>>,
    pub clean_compiled_state: CState,
}

impl<'a> Tower<'a> {
    pub fn new(tm: RunningTuringMachine<'a, CompiledUtmSpec<'a>>) -> Self {
        let clean_compiled_state = tm.spec.compile_state(State::Init);
        Self {
            base: TowerLevel {
                tm,
                total_steps: 0,
                max_head_pos: 0,
            },
            decoded: Vec::new(),
            clean_compiled_state,
        }
    }

    pub fn step(&mut self) -> RunningTMStatus {
        let prev_state = self.base.tm.state;
        let res = step(&mut self.base.tm);
        self.base.total_steps += 1;
        self.base.max_head_pos = max(self.base.max_head_pos, self.base.tm.pos);
        // eprintln!("step: {:?}", res);

        if self.base.tm.state == prev_state || self.base.tm.state != self.clean_compiled_state {
            // We didn't just transition into the clean state, so decoding isn't well-defined.
            return res;
        }

        let base_decompiled = self.base.tm.spec.decompile(&self.base.tm);

        let mut cur = &base_decompiled;
        let mut decoding = self.decoded.as_mut_slice();
        while let Some((next, rest)) = decoding.split_first_mut() {
            if !decode_into_level(cur, next) {
                // next level didn't enter Init, so we're done
                return res;
            }
            (cur, decoding) = (&next.tm, rest);
        }

        // we ran into the end of self.decoded, so we need to add a new level
        let new_level = TowerLevel {
            total_steps: 0,
            max_head_pos: 0,
            tm: MyUtmEncodingScheme::decode_with_orders(
                &*UTM_SPEC,
                &cur.tape,
                Some(&*crate::infinity::STATE_ORDER),
                Some(&*crate::infinity::SYMBOL_ORDER),
            )
            .expect("it should always be okay to decode a utm that just entered Init"),
        };
        self.decoded.push(new_level);

        return res;
    }
}

fn decode_into_level<'a>(tm: &UtmTm<'a>, dst: &mut UtmTowerLevel<'a>) -> bool {
    let decoded = MyUtmEncodingScheme::decode_with_orders(
        &*UTM_SPEC,
        &tm.tape,
        Some(&*crate::infinity::STATE_ORDER),
        Some(&*crate::infinity::SYMBOL_ORDER),
    )
    .expect("it should always be okay to decode a utm that just entered Init");
    let old_state = dst.tm.state;
    let new_state = decoded.state;
    dst.total_steps += 1;
    dst.max_head_pos = max(dst.max_head_pos, decoded.pos);
    dst.tm = decoded;

    return new_state != old_state && new_state == State::Init;
}
