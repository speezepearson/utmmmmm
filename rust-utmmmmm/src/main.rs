mod compiled;
mod infinity;
mod tm;
mod toy_machines;
mod utm;

use std::fmt::Debug;
use tm::{RunningTuringMachine, TapeExtender, TuringMachineSpec};
use utm::*;

use crate::infinity::InfiniteTapeExtender;

const RADIUS: usize = 30;

/// Format a tape view: 30 symbols on each side of the head, with ^ below.
fn tape_view<Spec: TuringMachineSpec<Symbol = Symbol>>(tm: &RunningTuringMachine<Spec>) -> String
where
    Spec::State: Debug,
{
    let mut top = String::new();
    let mut bot = String::new();

    let prefix = if tm.pos > RADIUS { "... " } else { "    " };
    top.push_str(prefix);
    bot.push_str("    ");

    for i in 0..(2 * RADIUS + 1) {
        let tape_idx = tm.pos as isize + i as isize - RADIUS as isize;
        let sym = if tape_idx < 0 {
            " ".to_string()
        } else {
            let idx = tape_idx as usize;
            if idx < tm.tape.len() {
                tm.tape[idx].to_string()
            } else {
                tm.spec.blank().to_string()
            }
        };
        top.push_str(&sym);

        if i == RADIUS {
            bot.push('^');
        } else {
            bot.push(' ');
        }
    }

    top.push_str(" ...");
    bot.push_str(&format!(" (state={:?}, pos={})", tm.state, tm.pos));

    format!("{}\n{}", top, bot)
}

fn print_tower<Spec: TuringMachineSpec<Symbol = Symbol>>(
    tm: &RunningTuringMachine<Spec>,
    steps: u64,
) where
    Spec::State: Debug,
{
    let utm = &*UTM_SPEC;

    eprintln!(
        "═══ {} steps ═══════════════════════════════════════",
        steps
    );

    eprintln!("Level 0 (outermost UTM):");
    eprintln!("{}", &format_tape(&tm.tape)[..1000]);

    eprintln!("Level 0 tape view:");
    eprintln!("{}", tape_view(tm));

    match MyUtmEncodingScheme::decode(utm, &tm.tape) {
        Ok(level1) => {
            eprintln!("Level 1 (simulated UTM):");
            eprintln!("{}", tape_view(&level1));

            match MyUtmEncodingScheme::decode(utm, &level1.tape) {
                Ok(level2) => {
                    eprintln!("Level 2 (simulated simulated UTM):");
                    eprintln!("{}", tape_view(&level2));
                }
                Err(e) => {
                    eprintln!("Level 2: (unable to decode: {})", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Level 1: (unable to decode: {})", e);
            eprintln!("Level 2: (unable to decode)");
        }
    }

    eprintln!();
}

fn main() {
    let utm = &*UTM_SPEC;

    let mut tm = RunningTuringMachine::new(utm);
    let extender = InfiniteTapeExtender;
    // Initialize tape with at least one cell
    extender.extend(&mut tm.tape, 1);

    let mut steps: u64 = 0;
    print_tower(&tm, steps);

    loop {
        if tm.pos >= tm.tape.len() {
            extender.extend(&mut tm.tape, tm.pos + 1);
        }
        let sym = tm.tape[tm.pos];
        if let Some((ns, nsym, dir)) = utm.get_transition(tm.state, sym) {
            tm.state = ns;
            tm.tape[tm.pos] = nsym;
            tm.pos = match dir {
                tm::Dir::Left => tm.pos.saturating_sub(1),
                tm::Dir::Right => tm.pos + 1,
            };
            steps += 1;
            if steps % 1_000_000 == 0 {
                print_tower(&tm, steps);
            }
        } else {
            break;
        }
    }

    print_tower(&tm, steps);
    let status = if utm.is_accepting(tm.state) {
        "accept"
    } else {
        "reject"
    };
    println!(
        "halted in state {:?} ({}) after {} steps",
        tm.state, status, steps
    );
}

#[cfg(test)]
mod tests;
