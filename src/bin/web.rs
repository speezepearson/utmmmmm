use std::sync::{Arc, Mutex};

use axum::{extract::State, response::Html, routing::get, Json, Router};
use serde::Serialize;
use utmmmmm::tm::{Dir, TuringMachine};
use utmmmmm::utm::{self, UtmSym, UtmState};

// ---- write1 TM (must match state_bits=2, symbol_bits=1) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum W1State { Start, Accept, Reject }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum W1Sym { Blank, One }

fn write1_tm() -> TuringMachine<W1State, W1Sym> {
    let mut t = std::collections::HashMap::new();
    t.insert(
        (W1State::Start, W1Sym::Blank),
        (W1State::Accept, W1Sym::One, Dir::Right),
    );
    TuringMachine {
        initial: W1State::Start,
        accept: W1State::Accept,
        reject: W1State::Reject,
        blank: W1Sym::Blank,
        transitions: t,
    }
}

// ---- Shared state ----

struct App {
    outer_sym_table: Vec<UtmSym>,
    inner_sym_table: Vec<W1Sym>,
    initial_tape: Vec<UtmSym>,
    sim: Mutex<UtmState>,
}

impl App {
    fn new() -> Self {
        let w1 = write1_tm();
        let inner_tape = utm::encode(&w1, &[]);
        let utm_tm = utm::build_utm_tm();
        let outer_tape = utm::encode(&utm_tm, &inner_tape);

        let outer_sym_table = build_sym_table(&utm_tm);
        let inner_sym_table = build_sym_table(&w1);

        let sim = Mutex::new(UtmState::new(&outer_tape));
        App { outer_sym_table, inner_sym_table, initial_tape: outer_tape, sim }
    }

    fn reset(&self) {
        *self.sim.lock().unwrap() = UtmState::new(&self.initial_tape);
    }
}

fn build_sym_table<S, A>(tm: &TuringMachine<S, A>) -> Vec<A>
where
    S: Eq + std::hash::Hash + Clone + std::fmt::Debug,
    A: Eq + std::hash::Hash + Clone + std::fmt::Debug,
{
    let mut symbols: Vec<A> = Vec::new();
    fn add_unique<T: Eq + Clone>(vec: &mut Vec<T>, item: &T) {
        if !vec.contains(item) { vec.push(item.clone()); }
    }
    for ((_, a), (_, a2, _)) in &tm.transitions {
        add_unique(&mut symbols, a);
        add_unique(&mut symbols, a2);
    }
    add_unique(&mut symbols, &tm.blank);
    symbols
}

type SharedApp = Arc<App>;

// ---- JSON types ----

#[derive(Serialize)]
struct TapeView {
    state: String,
    head: i64,
    tape: Vec<Option<String>>,
}

#[derive(Serialize)]
struct StateResponse {
    steps: u64,
    halted: bool,
    accepted: bool,
    outer: TapeView,
    middle: Option<TapeView>,
    inner: Option<TapeView>,
}

// ---- Symbol display ----

fn utmsym_char(s: UtmSym) -> char {
    match s {
        UtmSym::Zero => '0',
        UtmSym::One => '1',
        UtmSym::LBracket => '[',
        UtmSym::RBracket => ']',
        UtmSym::Pipe => '|',
        UtmSym::Semi => ';',
        UtmSym::Hash => '#',
        UtmSym::D => 'D',
        UtmSym::Blank => '_',
        UtmSym::Dot0 => 'a',
        UtmSym::Dot1 => 'b',
        UtmSym::MarkLBracket => '(',
    }
}

const WINDOW: i64 = 20;

fn build_tape_view(
    state: String,
    head: i64,
    len: i64,
    sym_fn: impl Fn(usize) -> String,
) -> TapeView {
    let tape: Vec<Option<String>> = (head - WINDOW..=head + WINDOW)
        .map(|pos| {
            if pos < 0 || pos >= len {
                None
            } else {
                Some(sym_fn(pos as usize))
            }
        })
        .collect();
    TapeView { state, head, tape }
}

fn w1sym_char(s: W1Sym) -> char {
    match s {
        W1Sym::Blank => '_',
        W1Sym::One => '1',
    }
}

// ---- Handlers ----

async fn handle_state(State(app): State<SharedApp>) -> Json<StateResponse> {
    let sim = app.sim.lock().unwrap();

    // === Outer: the interpreter simulating the UTM TM ===
    // Each outer data cell holds a (state_index, symbol_index).
    // Symbol indices map to outer_sym_table (UtmSym values).
    let outer = build_tape_view(
        format!("{}", sim.current_state),
        sim.head_pos as i64,
        sim.cells.len() as i64,
        |pos| {
            let idx = sim.cells[pos].1 as usize;
            app.outer_sym_table.get(idx)
                .map(|s| utmsym_char(*s).to_string())
                .unwrap_or("?".to_string())
        },
    );

    // === Reconstruct the UTM TM's tape as Vec<UtmSym> ===
    // This tape is encode(write1, []) as modified during simulation.
    let middle_tape: Vec<UtmSym> = sim.cells.iter()
        .map(|(_, sym_idx)| {
            app.outer_sym_table.get(*sym_idx as usize).copied().unwrap_or(UtmSym::Blank)
        })
        .collect();

    // === Middle: the UTM TM's tape shown as UtmSym chars ===
    // Decode the middle tape to find write1's head position in the data cells.
    // Then show the raw UtmSym tape centered on write1's head cell.
    let middle_decoded = utm::decode_running_state(&middle_tape);

    let middle = middle_decoded.as_ref().map(|md| {
        // Find where in the UtmSym tape the data region's head cell is.
        // The hash is somewhere in the middle tape; data starts after it.
        // Each data cell is [<state_bits><symbol_bits>|<symbol_bits>] =
        //   1 + state_bits + symbol_bits + 1 + symbol_bits + 1 chars... no,
        //   format is [ss|a] = 1 + 2 + 1 + 1 + 1 = 6 chars for state_bits=2,sym_bits=1.
        // But actually cell format is [ss|a] where ss = state_bits zeros/ones.
        // Let me just find the hash in middle_tape and compute.
        let hash_pos = middle_tape.iter().position(|s| matches!(s, UtmSym::Hash));
        let cell_width = 1 + md.state_bits + 1 + md.symbol_bits + 1; // [ss|a]
        let data_start = hash_pos.map(|h| h + 1).unwrap_or(0);
        let raw_head = data_start + md.head_pos * cell_width;

        build_tape_view(
            format!("{}", md.state),
            raw_head as i64,
            middle_tape.len() as i64,
            |pos| utmsym_char(middle_tape[pos]).to_string(),
        )
    });

    // === Inner: write1's tape decoded from the middle ===
    let inner = middle_decoded.as_ref().map(|md| {
        build_tape_view(
            format!("{}", md.state),
            md.head_pos as i64,
            md.tape_syms.len() as i64,
            |pos| {
                let idx = md.tape_syms[pos] as usize;
                app.inner_sym_table.get(idx)
                    .map(|s| w1sym_char(*s).to_string())
                    .unwrap_or("?".to_string())
            },
        )
    });

    Json(StateResponse {
        steps: sim.steps,
        halted: sim.halted,
        accepted: sim.accepted,
        outer,
        middle,
        inner,
    })
}

async fn handle_reset(State(app): State<SharedApp>) -> &'static str {
    app.reset();
    "ok"
}

async fn handle_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html>
<head>
<title>UTM on UTM</title>
<style>
body { background: #111; color: #0f0; font-family: monospace; font-size: 14px; padding: 20px; }
h1 { color: #0f0; font-size: 18px; }
h2 { color: #0a0; font-size: 14px; margin-top: 20px; margin-bottom: 4px; }
pre { margin: 0; line-height: 1.4; }
.tape-line { color: #0f0; letter-spacing: 1px; }
.head-line { color: #f80; letter-spacing: 1px; }
.controls { margin: 10px 0; }
button { background: #333; color: #0f0; border: 1px solid #0f0; padding: 5px 15px;
         cursor: pointer; font-family: monospace; margin-right: 10px; }
button:hover { background: #050; }
.info { color: #888; margin-bottom: 10px; }
</style>
</head>
<body>
<h1>UTM simulating UTM simulating write1</h1>
<div class="controls">
  <button onclick="doReset()">Reset</button>
  <span class="info" id="info">steps: 0</span>
</div>

<h2>Outer: UTM TM state (being simulated by interpreter)</h2>
<pre><span class="tape-line" id="outer-tape"></span>
<span class="head-line" id="outer-head"></span></pre>

<h2>Middle: UTM TM's tape (raw UtmSym encoding of write1)</h2>
<pre><span class="tape-line" id="middle-tape"></span>
<span class="head-line" id="middle-head"></span></pre>

<h2>Inner: write1's tape (decoded from middle)</h2>
<pre><span class="tape-line" id="inner-tape"></span>
<span class="head-line" id="inner-head"></span></pre>

<script>
function renderTape(tapeArr) {
    return tapeArr.map(s => {
        if (s === null) return ' ';
        return s.charAt(0);
    }).join('');
}

function renderHead(state, pos) {
    return ' '.repeat(20) + '^ (state=' + state + ') (pos=' + pos + ')';
}

function renderView(prefix, view) {
    if (!view) {
        document.getElementById(prefix + '-tape').textContent = '  (not yet decodable)';
        document.getElementById(prefix + '-head').textContent = '';
        return;
    }
    document.getElementById(prefix + '-tape').textContent = renderTape(view.tape);
    document.getElementById(prefix + '-head').textContent = renderHead(view.state, view.head);
}

async function refresh() {
    try {
        const resp = await fetch('/state');
        const data = await resp.json();
        let info = 'steps: ' + data.steps;
        if (data.halted) info += (data.accepted ? ' (ACCEPTED)' : ' (REJECTED)');
        document.getElementById('info').textContent = info;
        renderView('outer', data.outer);
        renderView('middle', data.middle);
        renderView('inner', data.inner);
    } catch(e) {
        console.error(e);
    }
}

async function doReset() {
    await fetch('/reset');
    refresh();
}

refresh();
setInterval(refresh, 200);
</script>
</body>
</html>
"##;

// ---- Main ----

#[tokio::main]
async fn main() {
    let app = Arc::new(App::new());
    let app_clone = Arc::clone(&app);

    // Stepping thread: step the UTM interpreter continuously
    std::thread::spawn(move || {
        loop {
            {
                let mut sim = app_clone.sim.lock().unwrap();
                if sim.halted {
                    drop(sim);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                for _ in 0..1000 {
                    if !sim.step() {
                        break;
                    }
                }
            }
            std::thread::yield_now();
        }
    });

    let router = Router::new()
        .route("/", get(handle_index))
        .route("/state", get(handle_state))
        .route("/reset", get(handle_reset))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Listening on http://localhost:8080");
    axum::serve(listener, router).await.unwrap();
}
