use utmmmmm::compiled::CompiledTuringMachineSpec;
use utmmmmm::infinity::InfiniteTape;
use utmmmmm::optimization_hints::make_my_utm_self_optimization_hints;
use utmmmmm::tm::{Dir, RunningTuringMachine, TuringMachineSpec};
use utmmmmm::utm::make_utm_spec;

const TAPE_EXTEND_CHUNK: usize = 4096;

fn main() {
    let max_steps: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000_000);

    let report_interval: u64 = 100_000_000;

    let optimization_hints = make_my_utm_self_optimization_hints();
    let utm_spec = make_utm_spec();
    let compiled = CompiledTuringMachineSpec::compile(&utm_spec).expect("UTM should compile");

    let mut tm = RunningTuringMachine::new(&compiled);
    let background = InfiniteTape::new(&utm_spec, &optimization_hints);

    let mut total_steps: u64 = 0;
    let mut inner_steps: u64 = 0;

    let start = std::time::Instant::now();
    let mut last_report = start;
    let mut next_report = report_interval;

    println!(
        "Running UTM benchmark for {} steps, reporting every {}...",
        max_steps, report_interval
    );

    // Pre-extend the tape so the inner loop has room to run.
    background.extend_compiled(&mut tm.tape, TAPE_EXTEND_CHUNK, &compiled);

    let mut state = tm.state;
    let mut pos = tm.pos;
    let mut prev_state = state;
    let mut halted = false;

    while total_steps < max_steps && !halted {
        // Ensure tape is large enough for a batch.
        if pos >= tm.tape.len() {
            tm.pos = pos;
            tm.state = state;
            let target = (pos + TAPE_EXTEND_CHUNK) & !(TAPE_EXTEND_CHUNK - 1);
            background.extend_compiled(&mut tm.tape, target, &compiled);
        }

        // Run a batch of steps in a tight loop with no reporting overhead.
        // The batch ends when we hit the tape boundary, step limit, or halt.
        let tape_end = tm.tape.len();
        let batch_limit = max_steps - total_steps;
        let mut batch_count: u64 = 0;

        while pos < tape_end && batch_count < batch_limit {
            let sym = tm.tape[pos];
            if let Some((ns, nsym, dir)) = compiled.get_transition(state, sym) {
                prev_state = state;
                state = ns;
                tm.tape[pos] = nsym;
                pos = match dir {
                    Dir::Left => pos.saturating_sub(1),
                    Dir::Right => pos + 1,
                };
                batch_count += 1;

                if compiled.is_tick_boundary(prev_state, state) {
                    inner_steps += 1;
                }
            } else {
                halted = true;
                break;
            }
        }

        total_steps += batch_count;

        if halted {
            println!(
                "UTM halted after {} steps ({} inner steps)",
                total_steps, inner_steps
            );
        }

        // Report if we've crossed a reporting boundary.
        while next_report <= total_steps {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(start).as_secs_f64();
            let interval_elapsed = now.duration_since(last_report).as_secs_f64();
            let steps_per_sec = report_interval as f64 / interval_elapsed;
            last_report = now;
            next_report += report_interval;

            println!(
                "{:>6.0}M outer steps | {:>8} inner steps | ratio {:>10.1} | {:.1}M steps/s | {:.1}s elapsed",
                total_steps as f64 / 1e6,
                inner_steps,
                if inner_steps > 0 { total_steps as f64 / inner_steps as f64 } else { f64::INFINITY },
                steps_per_sec / 1e6,
                elapsed,
            );
        }
    }

    tm.state = state;
    tm.pos = pos;

    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "\nDone: {} outer steps, {} inner steps, ratio {:.1}, {:.1}s total ({:.1}M steps/s)",
        total_steps,
        inner_steps,
        if inner_steps > 0 {
            total_steps as f64 / inner_steps as f64
        } else {
            f64::INFINITY
        },
        elapsed,
        total_steps as f64 / elapsed / 1e6,
    );
}
