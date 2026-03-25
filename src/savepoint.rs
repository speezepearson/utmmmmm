use std::collections::HashMap;
use std::io::Write as IoWrite;

use serde::{Deserialize, Serialize};

use crate::compiled::CompiledTuringMachineSpec;
use crate::compiled::{CState, CSymbol};
use crate::delta::current_overwrites;
use crate::infinity::InfiniteTapeExtender;
use crate::tm::{RunningTuringMachine, TapeExtender as _};
use crate::tm::SimpleTuringMachineSpec;
use crate::tower::{CompiledUtmSpec, Tower, TowerLevel};
use crate::utm::{State, Symbol, UTM_SPEC};

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub levels: Vec<TowerLevelJson>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TowerLevelJson {
    pub steps: u64,
    pub head_pos: usize,
    pub state: State,
    pub overwrites: HashMap<usize, Symbol>,
}

pub fn build_snapshot<'a>(
    tower: &'_ Tower<'a>,
    inf_extender: &mut InfiniteTapeExtender,
    reference: &mut Vec<Symbol>,
) -> Snapshot {
    let decompiled = tower.base.tm.spec.decompile(&tower.base.tm);
    inf_extender.extend(reference, decompiled.tape.len());
    let it = std::iter::once(TowerLevel {
        total_steps: tower.base.total_steps,
        tm: tower.base.tm.spec.decompile(&tower.base.tm),
    })
    .chain(tower.decoded.iter().map(|l| l.clone()));
    Snapshot {
        levels: it
            .map(|l| TowerLevelJson {
                steps: l.total_steps,
                head_pos: l.tm.pos,
                state: l.tm.state,
                overwrites: current_overwrites(&l.tm.tape, &reference)
                    .iter()
                    .map(|(&i, &s)| (i, s))
                    .collect::<HashMap<_, _>>(),
            })
            .collect(),
    }
}

pub fn save_savepoint(
    path: &str,
    tower: &Tower<'_>,
    reference: &mut Vec<Symbol>,
)
{
    let data = build_snapshot(tower, &mut InfiniteTapeExtender, reference);
    let tmp = format!("{}.tmp", path);
    let json = serde_json::to_string(&data).expect("serialize savepoint");
    let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp).expect("create savepoint"));
    f.write_all(json.as_bytes()).unwrap();
    drop(f);
    std::fs::rename(&tmp, path).expect("rename savepoint");
    eprintln!(
        "[{:?}] Saved savepoint at step {} to {}",
        std::time::Instant::now(),
        tower.base.total_steps,
        path
    );
}

pub fn load_savepoint<'a>(path: &str, spec: &'a CompiledUtmSpec<'a>) -> Option<Tower<'a>> {
    let data = std::fs::read(path).ok()?;
    let snapshot: Snapshot = serde_json::from_slice(&data).ok()?;
    let (snap_base, snap_decoded) = snapshot.levels.split_first().expect("savepoint should have at least one level");

    let mut tower = Tower::new(RunningTuringMachine::new(spec));

    tower.base.total_steps = snap_base.steps;
    tower.base.tm.state = spec.compile_state(snap_base.state);
    let mut tape = Vec::new();
    for (pos, sym) in snap_base.overwrites.iter() {
        InfiniteTapeExtender.extend(&mut tape, pos + 1);
        tape[*pos] = *sym;
    }
    tower.base.tm.tape = tape.iter().map(|&s| spec.compile_symbol(s)).collect();

    tower.decoded = snap_decoded.iter().map(|l| {
        let mut tape = Vec::new();
        for (pos, sym) in l.overwrites.iter() {
            InfiniteTapeExtender.extend(&mut tape, pos + 1);
            tape[*pos] = *sym;
        }
        TowerLevel {
            total_steps: l.steps,
            tm: RunningTuringMachine {
                spec: &*UTM_SPEC,
                pos: l.head_pos,
                state: l.state,
                tape,
            },
        }
    }).collect();
    Some(tower)
}
