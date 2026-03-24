use utmmmmm::savepoint::load_binary_savepoint;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("Usage: convert_savepoint <input-binary-savepoint> [output-json-savepoint]");
        eprintln!("If output is omitted, writes to stdout.");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let data = load_binary_savepoint(input_path).unwrap_or_else(|| {
        eprintln!("Failed to load binary savepoint from {}", input_path);
        std::process::exit(1);
    });

    let json = serde_json::to_string(&data).expect("serialize savepoint");

    if args.len() == 3 {
        let output_path = &args[2];
        std::fs::write(output_path, json.as_bytes()).unwrap_or_else(|e| {
            eprintln!("Failed to write {}: {}", output_path, e);
            std::process::exit(1);
        });
        eprintln!(
            "Converted {} -> {} (step {}, {} guest steps, tape len {})",
            input_path, output_path, data.total_steps, data.guest_steps, data.tape.len()
        );
    } else {
        println!("{}", json);
    }
}
