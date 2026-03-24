use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::Write as IoWrite;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tiny_http::{Header, Response, Server};
use utmmmmm::compiled::{CState, CompiledTapeExtender, CompiledTuringMachineSpec};
use utmmmmm::delta::{compute_new_overwrites, current_overwrites, ClientLevelState};
use utmmmmm::infinity::InfiniteTapeExtender;
use utmmmmm::savepoint::{load_savepoint, save_savepoint};
use utmmmmm::tm::{
    Dir, RunningTuringMachine, SimpleTuringMachineSpec, TapeExtender, TuringMachineSpec,
};
use utmmmmm::utm::{State, Symbol, UTM_SPEC};

// ── Snapshot: shared between tower thread and SSE client threads ──

struct Snapshot {
    total_steps: u64,
    guest_steps: u64,
    steps_per_sec: f64,
    head_pos: usize,
    max_head_pos: usize,
    state: String,
    tape_len: usize,
    overwrites: HashMap<usize, Symbol>,
}

type SseClient = mpsc::Sender<Arc<Snapshot>>;
type SseClients = Arc<Mutex<Vec<SseClient>>>;

// ── JSON event types ──

#[derive(Serialize)]
struct TotalEventJson {
    #[serde(rename = "type")]
    event_type: &'static str,
    steps: u64,
    guest_steps: u64,
    steps_per_sec: f64,
    unblemished: String,
    utm_states: Vec<String>,
    utm_symbol_chars: String,
    state: String,
    head_pos: usize,
    max_head_pos: usize,
    tape: String,
    tape_len: usize,
}

#[derive(Serialize)]
struct DeltaEventJson {
    #[serde(rename = "type")]
    event_type: &'static str,
    total_steps: u64,
    guest_steps: u64,
    steps_per_sec: f64,
    state: String,
    head_pos: usize,
    max_head_pos: usize,
    new_overwrites: Vec<(usize, String)>,
    tape_len: usize,
}

// ── Tape reconstruction (for total event) ──

fn reconstruct_tape(
    unblemished: &str,
    overwrites: &HashMap<usize, Symbol>,
    tape_len: usize,
) -> String {
    let ub = unblemished.as_bytes();
    let mut bytes = Vec::with_capacity(tape_len);
    for i in 0..tape_len {
        if let Some(&sym) = overwrites.get(&i) {
            let mut s = String::new();
            write!(s, "{}", sym).unwrap();
            bytes.push(s.as_bytes()[0]);
        } else if i < ub.len() {
            bytes.push(ub[i]);
        } else {
            bytes.push(b'_');
        }
    }
    String::from_utf8(bytes).unwrap()
}

// ── Build snapshot from decompiled L0 machine ──

fn build_snapshot(
    decompiled: &RunningTuringMachine<SimpleTuringMachineSpec<State, Symbol>>,
    max_head_pos: usize,
    total_steps: u64,
    guest_steps: u64,
    steps_per_sec: f64,
    inf_extender: &mut InfiniteTapeExtender,
    reference: &mut Vec<Symbol>,
) -> Snapshot {
    inf_extender.extend(reference, decompiled.tape.len());
    Snapshot {
        total_steps,
        guest_steps,
        steps_per_sec,
        head_pos: decompiled.pos,
        max_head_pos,
        state: format!("{:?}", decompiled.state),
        tape_len: decompiled.tape.len(),
        overwrites: current_overwrites(&decompiled.tape, reference),
    }
}

fn publish(
    latest: &Mutex<Option<Arc<Snapshot>>>,
    sse_clients: &Mutex<Vec<SseClient>>,
    snap: Arc<Snapshot>,
) {
    *latest.lock().unwrap() = Some(Arc::clone(&snap));
    let mut clients = sse_clients.lock().unwrap();
    clients.retain(|tx| tx.send(Arc::clone(&snap)).is_ok());
}

// ── Main simulation thread ──

fn sim_thread(
    latest: Arc<Mutex<Option<Arc<Snapshot>>>>,
    sse_clients: SseClients,
    savepoint_path: Option<String>,
) {
    let utm = &*UTM_SPEC;
    let compiled = CompiledTuringMachineSpec::compile(utm).expect("UTM should compile");

    let init_cstate = compiled
        .original_states
        .iter()
        .position(|&s| s == State::Init)
        .map(|i| CState(i as u8))
        .expect("Init state should exist");

    let mut tm = RunningTuringMachine::new(&compiled);
    let mut extender = CompiledTapeExtender::new(&compiled, Box::new(InfiniteTapeExtender));
    extender.extend(&mut tm.tape, 1);

    let mut total_steps: u64 = 0;
    let mut guest_steps: u64 = 0;

    if let Some(ref sp_path) = savepoint_path {
        if let Some((sp_steps, sp_guest, sp_state, sp_pos, sp_tape)) = load_savepoint(sp_path) {
            total_steps = sp_steps;
            guest_steps = sp_guest;
            tm.state = sp_state;
            tm.pos = sp_pos;
            tm.tape = sp_tape;
            let tape_len = tm.tape.len();
            extender.extend(&mut tm.tape, tape_len);
            eprintln!(
                "Loaded savepoint from {}: step {}, {} guest steps, tape len {}",
                sp_path,
                total_steps,
                guest_steps,
                tm.tape.len()
            );
        }
    }

    let mut base_max_pos: usize = tm.pos;
    let mut inf_extender = InfiniteTapeExtender;
    let mut last_savepoint_step = total_steps;

    // Reference tape for overwrite comparison
    let mut reference: Vec<Symbol> = Vec::new();

    let snapshot_interval = Duration::from_millis(30);
    let mut last_snapshot = Instant::now();
    let start_time = Instant::now();
    let mut prev_cstate = tm.state;

    // Profiling: time spent doing things other than the hot loop
    let mut total_overhead = Duration::ZERO;
    let mut last_profile_print = Instant::now();

    // Initial snapshot
    {
        let decompiled = compiled.decompile(&tm);
        let snap = Arc::new(build_snapshot(
            &decompiled,
            base_max_pos,
            total_steps,
            guest_steps,
            0.0,
            &mut inf_extender,
            &mut reference,
        ));
        publish(&latest, &sse_clients, snap);
    }

    loop {
        if tm.pos >= tm.tape.len() {
            extender.extend(&mut tm.tape, tm.pos + 1);
        }

        let sym = tm.tape[tm.pos];
        if let Some((ns, nsym, dir)) = compiled.get_transition(tm.state, sym) {
            tm.state = ns;
            tm.tape[tm.pos] = nsym;
            tm.pos = match dir {
                Dir::Left => tm.pos.saturating_sub(1),
                Dir::Right => tm.pos + 1,
            };
            total_steps += 1;
            if tm.pos > base_max_pos {
                base_max_pos = tm.pos;
            }
        } else {
            // Halted
            let decompiled = compiled.decompile(&tm);
            let snap = Arc::new(build_snapshot(
                &decompiled,
                base_max_pos,
                total_steps,
                guest_steps,
                0.0,
                &mut inf_extender,
                &mut reference,
            ));
            publish(&latest, &sse_clients, snap);
            if let Some(ref sp_path) = savepoint_path {
                save_savepoint(sp_path, total_steps, guest_steps, &tm);
            }
            return;
        }

        if tm.state != prev_cstate {
            if tm.state == init_cstate {
                guest_steps += 1;
            }
            prev_cstate = tm.state;
        }

        if total_steps % 100_000 == 0 {
            let overhead_start_at = Instant::now();
            if let Some(ref sp_path) = savepoint_path {
                if total_steps - last_savepoint_step >= 1_000_000_000 {
                    save_savepoint(sp_path, total_steps, guest_steps, &tm);
                    last_savepoint_step = total_steps;
                }
            }

            if last_snapshot.elapsed() >= snapshot_interval {
                let decompiled = compiled.decompile(&tm);
                let wall_secs = start_time.elapsed().as_secs_f64().max(0.001);
                let steps_per_sec = total_steps as f64 / wall_secs / 1_000_000.0;
                let snap = Arc::new(build_snapshot(
                    &decompiled,
                    base_max_pos,
                    total_steps,
                    guest_steps,
                    steps_per_sec,
                    &mut inf_extender,
                    &mut reference,
                ));
                publish(&latest, &sse_clients, snap);
                last_snapshot = Instant::now();
            }

            if last_profile_print.elapsed() >= Duration::from_secs(10) {
                let elapsed = last_profile_print.elapsed();
                eprintln!(
                    "[profile] snapshot block: {:.1}ms / {:.1}s ({:.2}%)",
                    total_overhead.as_secs_f64() * 1000.0,
                    elapsed.as_secs_f64(),
                    total_overhead.as_secs_f64() / elapsed.as_secs_f64() * 100.0,
                );
                last_profile_print = Instant::now();
            }

            total_overhead += overhead_start_at.elapsed();
        }
    }
}

// ── SSE client thread ──

fn sse_client_thread(
    rx: mpsc::Receiver<Arc<Snapshot>>,
    latest: Arc<Mutex<Option<Arc<Snapshot>>>>,
    unblemished_str: Arc<String>,
    unblemished_syms: Arc<Vec<Symbol>>,
    utm_states: Arc<Vec<String>>,
    utm_symbol_chars: Arc<String>,
    mut writer: Box<dyn IoWrite + Send>,
) {
    // Get initial snapshot (prefer stored latest, fall back to waiting on channel)
    let initial = {
        let stored = latest.lock().unwrap().as_ref().map(Arc::clone);
        match stored {
            Some(snap) => snap,
            None => match rx.recv() {
                Ok(snap) => snap,
                Err(_) => return,
            },
        }
    };

    // Send total event
    let tape_end = initial.max_head_pos + 10;
    let total = TotalEventJson {
        event_type: "total",
        steps: initial.total_steps,
        guest_steps: initial.guest_steps,
        steps_per_sec: initial.steps_per_sec,
        unblemished: (*unblemished_str).clone(),
        utm_states: (*utm_states).clone(),
        utm_symbol_chars: (*utm_symbol_chars).clone(),
        state: initial.state.clone(),
        head_pos: initial.head_pos,
        max_head_pos: initial.max_head_pos,
        tape: reconstruct_tape(&unblemished_str, &initial.overwrites, tape_end),
        tape_len: initial.tape_len,
    };
    let json = serde_json::to_string(&total).unwrap();
    if write!(writer, "data: {}\n\n", json).is_err() || writer.flush().is_err() {
        return;
    }

    // Initialize client state from the total snapshot's overwrites
    let mut client_state = ClientLevelState {
        overwrites: initial.overwrites.clone(),
    };

    // Stream delta events
    while let Ok(snapshot) = rx.recv() {
        let new_overwrites =
            compute_new_overwrites(&snapshot.overwrites, &mut client_state, &unblemished_syms);
        let delta = DeltaEventJson {
            event_type: "delta",
            total_steps: snapshot.total_steps,
            guest_steps: snapshot.guest_steps,
            steps_per_sec: snapshot.steps_per_sec,
            state: snapshot.state.clone(),
            head_pos: snapshot.head_pos,
            max_head_pos: snapshot.max_head_pos,
            new_overwrites,
            tape_len: snapshot.tape_len,
        };
        let json = serde_json::to_string(&delta).unwrap();
        if write!(writer, "data: {}\n\n", json).is_err() || writer.flush().is_err() {
            break;
        }
    }
}

// ── HTTP ──

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".wasm") {
        "application/wasm"
    } else {
        "application/octet-stream"
    }
}

fn get_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1].clone())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let savepoint_path = get_flag(&args, "--savepoint");
    let port = get_flag(&args, "--port")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    // Pre-compute the unblemished infinite tape (1M symbols)
    let unblemished_syms = {
        let mut syms: Vec<Symbol> = Vec::new();
        InfiniteTapeExtender.extend(&mut syms, 1_000_000);
        Arc::new(syms)
    };
    let unblemished_str: Arc<String> =
        Arc::new(unblemished_syms.iter().map(|s| format!("{}", s)).collect());

    // Pre-compute UTM metadata for client-side decoding
    let utm = &*UTM_SPEC;
    let utm_states: Arc<Vec<String>> =
        Arc::new(utm.iter_states().map(|s| format!("{:?}", s)).collect());
    let utm_symbol_chars: Arc<String> =
        Arc::new(utm.iter_symbols().map(|s| format!("{}", s)).collect());

    let latest: Arc<Mutex<Option<Arc<Snapshot>>>> = Arc::new(Mutex::new(None));
    let sse_clients: SseClients = Arc::new(Mutex::new(Vec::new()));

    // Start simulation background thread
    let latest_clone = Arc::clone(&latest);
    let sse_clone = Arc::clone(&sse_clients);
    thread::spawn(move || sim_thread(latest_clone, sse_clone, savepoint_path));

    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr).expect("Failed to start HTTP server");
    eprintln!("Serving on http://localhost:{}", port);

    // Find ui/dist relative to the cargo manifest directory or current dir
    let dist_dir = if Path::new("ui/dist").is_dir() {
        Path::new("ui/dist").to_path_buf()
    } else if Path::new("dist").is_dir() {
        Path::new("dist").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/dist")
    };

    for request in server.incoming_requests() {
        let url = request.url().to_string();

        if url == "/api/tower" {
            // SSE: grab the raw socket and stream events
            let latest_c = Arc::clone(&latest);
            let clients_c = Arc::clone(&sse_clients);
            let ub_str = Arc::clone(&unblemished_str);
            let ub_syms = Arc::clone(&unblemished_syms);
            let utm_st = Arc::clone(&utm_states);
            let utm_sc = Arc::clone(&utm_symbol_chars);
            let mut writer = request.into_writer();

            // Write HTTP response headers for SSE
            let header_ok = write!(
                writer,
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/event-stream\r\n\
                 Cache-Control: no-cache\r\n\
                 Connection: keep-alive\r\n\
                 \r\n"
            )
            .is_ok()
                && writer.flush().is_ok();

            if !header_ok {
                continue;
            }

            // Create channel and register BEFORE reading latest snapshot,
            // so we don't miss any broadcasts between reading latest and subscribing.
            let (tx, rx) = mpsc::channel();
            clients_c.lock().unwrap().push(tx);

            thread::spawn(move || {
                sse_client_thread(rx, latest_c, ub_str, ub_syms, utm_st, utm_sc, writer);
            });
            continue;
        }

        // Serve static files from ui/dist/
        let file_path = if url == "/" {
            dist_dir.join("index.html")
        } else {
            dist_dir.join(url.trim_start_matches('/'))
        };

        if file_path.is_file() {
            match std::fs::read(&file_path) {
                Ok(data) => {
                    let ct = content_type_for(file_path.to_str().unwrap_or(""));
                    let response = Response::from_data(data)
                        .with_header(Header::from_bytes("Content-Type", ct).unwrap());
                    let _ = request.respond(response);
                }
                Err(_) => {
                    let _ = request.respond(Response::from_string("500").with_status_code(500));
                }
            }
        } else {
            // SPA fallback: serve index.html for unmatched routes
            let index = dist_dir.join("index.html");
            match std::fs::read(&index) {
                Ok(data) => {
                    let response = Response::from_data(data).with_header(
                        Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap(),
                    );
                    let _ = request.respond(response);
                }
                Err(_) => {
                    let _ = request.respond(Response::from_string("404").with_status_code(404));
                }
            }
        }
    }
}
