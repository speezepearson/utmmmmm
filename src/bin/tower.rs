use utmmmmm::compiled::{CState, CompiledTapeExtender, CompiledTuringMachineSpec};
use utmmmmm::infinity::InfiniteTapeExtender;
use utmmmmm::savepoint::{load_savepoint, save_savepoint};
use utmmmmm::tm::{Dir, RunningTuringMachine, TuringMachineSpec};
use utmmmmm::tower::{colorize_ansi, format_tower, update_tower, TowerLevel};
use utmmmmm::utm::{State, UTM_SPEC};

fn get_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1].clone())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let savepoint_path = get_flag(&args, "--savepoint");

    let utm = &*UTM_SPEC;
    let compiled = CompiledTuringMachineSpec::compile(utm).expect("UTM should compile");

    // Find the CState corresponding to State::Init
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

    // Load savepoint if it exists
    if let Some(ref sp_path) = savepoint_path {
        if let Some((sp_steps, sp_state, sp_pos, sp_tape)) = load_savepoint(sp_path) {
            total_steps = sp_steps;
            tm.state = sp_state;
            tm.pos = sp_pos;
            tm.tape = sp_tape;
            // Sync the extender's shadow tape to the loaded tape length
            let tape_len = tm.tape.len();
            extender.extend(&mut tm.tape, tape_len);
            eprintln!(
                "Loaded savepoint from {}: step {}, tape len {}",
                sp_path,
                total_steps,
                tm.tape.len()
            );
        }
    }

    let mut inf_extender = InfiniteTapeExtender;
    let mut base_max_pos: usize = tm.pos;

    // Initialize tower
    let mut tower: Vec<TowerLevel> = vec![TowerLevel::new(compiled.decompile(&tm))];
    if tm.state == init_cstate {
        update_tower(utm, &mut tower, &mut inf_extender);
    }
    tower[0].max_head_pos = base_max_pos;
    eprint!(
        "{}",
        colorize_ansi(&format_tower(
            &mut tower,
            total_steps,
            utm,
            &mut inf_extender
        ))
    );

    let print_interval = std::time::Duration::from_millis(100);
    let mut last_print = std::time::Instant::now();
    let start_time = std::time::Instant::now();
    let mut prev_cstate = tm.state;
    let mut last_savepoint_step = total_steps;

    loop {
        // Extend tape if needed
        if tm.pos >= tm.tape.len() {
            extender.extend(&mut tm.tape, tm.pos + 1);
        }

        // Step
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
            tower[0].update_machine(compiled.decompile(&tm));
            tower[0].max_head_pos = base_max_pos;
            update_tower(utm, &mut tower, &mut inf_extender);
            eprint!(
                "{}",
                colorize_ansi(&format_tower(
                    &mut tower,
                    total_steps,
                    utm,
                    &mut inf_extender
                ))
            );
            let status = if compiled.is_accepting(tm.state) {
                "accept"
            } else {
                "reject"
            };
            println!(
                "halted ({}) in state {:?} after {} UTM steps",
                status, tower[0].machine.state, total_steps
            );
            if let Some(ref sp_path) = savepoint_path {
                save_savepoint(sp_path, total_steps, &tm);
            }
            return;
        }

        // Detect Init entry
        if tm.state != prev_cstate {
            if tm.state == init_cstate {
                tower[0].update_machine(compiled.decompile(&tm));
                tower[0].max_head_pos = base_max_pos;
                update_tower(utm, &mut tower, &mut inf_extender);
            }
            prev_cstate = tm.state;
        }

        // Periodic checks (every 100K steps to avoid syscall overhead)
        if total_steps % 100_000 == 0 {
            // Savepoint every 1B steps
            if let Some(ref sp_path) = savepoint_path {
                if total_steps - last_savepoint_step >= 1_000_000_000 {
                    save_savepoint(sp_path, total_steps, &tm);
                    last_savepoint_step = total_steps;
                }
            }

            // Print every 0.1s
            if last_print.elapsed() >= print_interval {
                tower[0].update_machine(compiled.decompile(&tm));
                tower[0].max_head_pos = base_max_pos;
                let wall_secs = start_time.elapsed().as_secs_f64().max(0.001);
                eprint!(
                    "{}  ({:.1}M steps/s)\n",
                    colorize_ansi(&format_tower(
                        &mut tower,
                        total_steps,
                        utm,
                        &mut inf_extender
                    )),
                    total_steps as f64 / wall_secs / 1_000_000.0
                );
                last_print = std::time::Instant::now();
            }
        }
    }
}
