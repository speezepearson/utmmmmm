use std::io::Write as IoWrite;

use serde::{Deserialize, Serialize};

use crate::compiled::CompiledTuringMachineSpec;
use crate::compiled::{CState, CSymbol};
use crate::tm::RunningTuringMachine;
use crate::tm::SimpleTuringMachineSpec;

#[derive(Serialize, Deserialize)]
pub struct SavepointData {
    pub total_steps: u64,
    pub guest_steps: u64,
    pub state: u8,
    pub pos: u64,
    pub tape: Vec<u8>,
}

pub fn save_savepoint<S: Copy + std::fmt::Debug, Y: Copy + std::fmt::Debug>(
    path: &str,
    total_steps: u64,
    guest_steps: u64,
    tm: &RunningTuringMachine<CompiledTuringMachineSpec<SimpleTuringMachineSpec<S, Y>>>,
) where
    S: std::hash::Hash + Eq,
    Y: std::hash::Hash + Eq + std::fmt::Display,
{
    let data = SavepointData {
        total_steps,
        guest_steps,
        state: tm.state.0,
        pos: tm.pos as u64,
        tape: tm.tape.iter().map(|s| s.0).collect(),
    };
    let tmp = format!("{}.tmp", path);
    let json = serde_json::to_string(&data).expect("serialize savepoint");
    let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp).expect("create savepoint"));
    f.write_all(json.as_bytes()).unwrap();
    drop(f);
    std::fs::rename(&tmp, path).expect("rename savepoint");
    eprintln!(
        "[{:?}] Saved savepoint at step {} to {}",
        std::time::Instant::now(),
        total_steps,
        path
    );
}

pub fn load_savepoint(path: &str) -> Option<(u64, u64, CState, usize, Vec<CSymbol>)> {
    let data = std::fs::read(path).ok()?;
    let sp: SavepointData = serde_json::from_slice(&data).ok()?;
    let tape: Vec<CSymbol> = sp.tape.iter().map(|&b| CSymbol(b)).collect();
    Some((
        sp.total_steps,
        sp.guest_steps,
        CState(sp.state),
        sp.pos as usize,
        tape,
    ))
}

/// Load a savepoint from the old binary format.
pub fn load_binary_savepoint(path: &str) -> Option<SavepointData> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 33 {
        return None;
    }
    let total_steps = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let guest_steps = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let state = data[16];
    let pos = u64::from_le_bytes(data[17..25].try_into().unwrap());
    let tape_len = u64::from_le_bytes(data[25..33].try_into().unwrap()) as usize;
    if data.len() < 33 + tape_len {
        return None;
    }
    let tape = data[33..33 + tape_len].to_vec();
    Some(SavepointData {
        total_steps,
        guest_steps,
        state,
        pos,
        tape,
    })
}
