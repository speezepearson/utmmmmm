//! WASM bindings for UTM encoding/decoding.
//!
//! Exposes `encode` and `decode` to JavaScript via wasm-bindgen.
//! The JS side passes a JSON-serialised TuringMachineSpec (the same
//! shape exported by `json_export`) plus the tape contents, and gets
//! back the encoded/decoded tape as a string.

use std::collections::{HashMap, HashSet};

use wasm_bindgen::prelude::*;

use crate::tm::{Dir, RunningTuringMachine, SimpleTuringMachineSpec};
use crate::utm::{MyUtmEncodingScheme, UtmEncodingScheme};

// ── JSON DTOs (matching the TypeScript/json_export shape) ──

#[derive(serde::Deserialize)]
struct JsonSpec {
    #[serde(rename = "allStates")]
    all_states: Vec<String>,
    #[serde(rename = "allSymbols")]
    all_symbols: Vec<String>,
    initial: String,
    #[serde(rename = "acceptingStates")]
    accepting_states: Vec<String>,
    blank: String,
    /// state -> symbol_name -> [new_state, new_symbol_name, dir]
    rules: HashMap<String, HashMap<String, (String, String, String)>>,
    /// symbol_name -> display char  (e.g. "Zero" -> "0")
    #[serde(rename = "symbolChars")]
    symbol_chars: HashMap<String, String>,
}

/// Build a `SimpleTuringMachineSpec<usize, usize>` from the JSON DTO.
/// States and symbols are represented as indices into their respective arrays.
fn build_spec(
    json: &JsonSpec,
) -> (
    SimpleTuringMachineSpec<usize, usize>,
    Vec<String>,   // state names
    Vec<String>,   // symbol display chars (ordered like allSymbols)
    HashMap<String, usize>, // symbol name -> index
) {
    let state_idx: HashMap<&str, usize> = json
        .all_states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.as_str(), i))
        .collect();

    // symbol_chars maps symbol-name -> display char
    // allSymbols contains symbol names
    let sym_name_to_idx: HashMap<String, usize> = json
        .all_symbols
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i))
        .collect();

    let display_chars: Vec<String> = json
        .all_symbols
        .iter()
        .map(|name| {
            json.symbol_chars
                .get(name)
                .cloned()
                .unwrap_or_else(|| name.clone())
        })
        .collect();

    let initial = state_idx[json.initial.as_str()];
    let blank_name = &json.blank;
    let blank = sym_name_to_idx[blank_name];

    let accepting: HashSet<usize> = json
        .accepting_states
        .iter()
        .map(|s| state_idx[s.as_str()])
        .collect();

    let mut transitions: HashMap<(usize, usize), (usize, usize, Dir)> = HashMap::new();
    for (st_name, sym_map) in &json.rules {
        let st = state_idx[st_name.as_str()];
        for (sym_name, (nst_name, nsym_name, dir_str)) in sym_map {
            let sym = sym_name_to_idx[sym_name];
            let nst = state_idx[nst_name.as_str()];
            let nsym = sym_name_to_idx[nsym_name];
            let dir = match dir_str.as_str() {
                "L" => Dir::Left,
                "R" => Dir::Right,
                _ => panic!("invalid dir: {}", dir_str),
            };
            transitions.insert((st, sym), (nst, nsym, dir));
        }
    }

    let all_states: Vec<usize> = (0..json.all_states.len()).collect();
    let all_symbols: Vec<usize> = (0..json.all_symbols.len()).collect();

    let spec = SimpleTuringMachineSpec {
        initial,
        accepting,
        blank,
        transitions,
        all_states,
        all_symbols,
    };

    (spec, json.all_states.clone(), display_chars, sym_name_to_idx)
}

/// Encode a guest TM into a UTM tape.
///
/// - `spec_json`: JSON string of the machine spec (same format as machine-specs.json entries)
/// - `tape_str`: the tape contents as display characters (e.g. "01011")
///
/// Returns the encoded UTM tape as a string of UTM symbol display characters.
#[wasm_bindgen]
pub fn encode(spec_json: &str, tape_str: &str) -> Result<String, JsError> {
    let json: JsonSpec = serde_json::from_str(spec_json)
        .map_err(|e| JsError::new(&format!("invalid spec JSON: {}", e)))?;

    let (spec, _state_names, display_chars, _sym_name_to_idx) = build_spec(&json);

    // Build reverse map: display char -> symbol index
    let char_to_idx: HashMap<&str, usize> = display_chars
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();

    // Parse the tape string into symbol indices
    let tape: Vec<usize> = tape_str
        .chars()
        .map(|c| {
            let s = c.to_string();
            *char_to_idx
                .get(s.as_str())
                .unwrap_or_else(|| panic!("unknown symbol char: {:?}", c))
        })
        .collect();

    let mut tm = RunningTuringMachine::new(&spec);
    tm.tape = if tape.is_empty() {
        vec![spec.blank]
    } else {
        tape
    };

    let encoded = MyUtmEncodingScheme::encode(&tm);

    // Convert UTM symbols to their display chars
    let result: String = encoded.iter().map(|s| s.to_string()).collect();
    Ok(result)
}

/// Decode a UTM tape back into guest TM state.
///
/// - `spec_json`: JSON string of the guest machine spec
/// - `utm_tape_str`: the UTM tape as a string of UTM symbol chars
///
/// Returns a JSON string: `{ "state": "...", "tape": "...", "pos": N }`
#[wasm_bindgen]
pub fn decode(spec_json: &str, utm_tape_str: &str) -> Result<String, JsError> {
    let json: JsonSpec = serde_json::from_str(spec_json)
        .map_err(|e| JsError::new(&format!("invalid spec JSON: {}", e)))?;

    let (spec, state_names, display_chars, _sym_name_to_idx) = build_spec(&json);

    // Parse UTM tape string into UTM symbols
    let utm_tape: Vec<crate::utm::Symbol> = utm_tape_str
        .chars()
        .map(|c| {
            let s = c.to_string();
            serde_json::from_str::<crate::utm::Symbol>(&format!("\"{}\"", s))
                .unwrap_or_else(|_| panic!("unknown UTM symbol char: {:?}", c))
        })
        .collect();

    let decoded = MyUtmEncodingScheme::decode(&spec, &utm_tape)
        .map_err(|e| JsError::new(&format!("decode failed: {}", e)))?;

    let state_name = &state_names[decoded.state];
    let tape_str: String = decoded
        .tape
        .iter()
        .map(|&sym_idx| display_chars[sym_idx].as_str())
        .collect();

    let result = serde_json::json!({
        "state": state_name,
        "tape": tape_str,
        "pos": decoded.pos,
    });

    Ok(result.to_string())
}
