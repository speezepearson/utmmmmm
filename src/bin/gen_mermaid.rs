use utmmmmm::tm::TuringMachineSpec;
use utmmmmm::utm::make_utm_spec;

fn cluster_for(state_name: &str) -> (&'static str, &'static str) {
    // Returns (cluster_id, cluster_label)
    if state_name.starts_with("Init") {
        ("init", "Phase 0: Init")
    } else if state_name == "MarkRule" || state_name == "MarkRuleNoMatch" {
        ("mark_rule", "Phase 1: Mark Rule")
    } else if state_name.starts_with("CmpSt")
        || state_name.starts_with("Stm")
        || state_name.starts_with("Stf")
        || state_name == "StMatchCleanup"
        || state_name == "SymSkipState"
    {
        ("cmp_state", "Phase 2: Compare State")
    } else if state_name.starts_with("CmpSym")
        || state_name.starts_with("Symf")
        || state_name == "SymMatchCleanup"
        || state_name.starts_with("Smc")
    {
        ("cmp_sym", "Phase 3: Compare Symbol")
    } else if state_name.starts_with("CpNst") || state_name == "ApplyReadNst" {
        ("cp_nst", "Phase 4: Copy New State")
    } else if state_name.starts_with("CpNsym") {
        ("cp_nsym", "Phase 5: Copy New Symbol")
    } else if state_name.starts_with("Rd") || state_name == "ReadDir" {
        ("read_dir", "Phase 6: Read Direction")
    } else if state_name.starts_with("Mr") || state_name == "MoveRight" {
        ("move_right", "Move Right")
    } else if state_name.starts_with("Ml") || state_name == "MoveLeft" {
        ("move_left", "Move Left")
    } else if state_name == "DoneSeekHome" {
        ("seek_home", "Phase 7: Seek Home")
    } else if state_name.starts_with("ChkAcc") || state_name.starts_with("Nm") {
        ("chk_acc", "Phase 8: Check Accept")
    } else if state_name.starts_with("Acc") || state_name == "Accept" {
        ("accept", "Accept")
    } else if state_name.starts_with("Rej") || state_name == "Reject" {
        ("reject", "Reject")
    } else {
        ("other", "Other")
    }
}

fn main() {
    let spec = make_utm_spec();

    // Collect all states and their clusters
    let mut clusters: std::collections::BTreeMap<&str, (String, Vec<String>)> =
        std::collections::BTreeMap::new();

    for state in spec.iter_states() {
        let name = format!("{:?}", state);
        let (cluster_id, cluster_label) = cluster_for(&name);
        clusters
            .entry(cluster_id)
            .or_insert_with(|| (cluster_label.to_string(), Vec::new()))
            .1
            .push(name);
    }

    println!("flowchart TD");

    // Emit subgraphs
    for (cluster_id, (label, states)) in &clusters {
        println!("    subgraph {} [\"{}\" ]", cluster_id, label);
        for state in states {
            println!("        {}", state);
        }
        println!("    end");
    }

    println!();

    // Emit edges
    for (state, sym, next_state, next_sym, dir) in spec.iter_rules() {
        // Skip self-loops that don't change the symbol
        if state == next_state && sym == next_sym {
            continue;
        }

        let dir_str = match dir {
            utmmmmm::tm::Dir::Left => "L",
            utmmmmm::tm::Dir::Right => "R",
        };

        let label = format!("{}/{},{}", sym, next_sym, dir_str);
        println!(
            "    {:?} -->|\"{}\"| {:?}",
            state, label, next_state
        );
    }
}
