use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = get_flag(&args, "--input").unwrap_or_else(|| {
        eprintln!("Usage: render-state-log --input PATH --output PATH [--width N] [--height N]");
        std::process::exit(1);
    });
    let output = get_flag(&args, "--output").unwrap_or_else(|| {
        eprintln!("Usage: render-state-log --input PATH --output PATH [--width N] [--height N]");
        std::process::exit(1);
    });
    let width: usize = get_flag(&args, "--width")
        .map(|s| s.parse().expect("--width must be a number"))
        .unwrap_or(1920);
    let height: usize = get_flag(&args, "--height")
        .map(|s| s.parse().expect("--height must be a number"))
        .unwrap_or(1080);

    let mut file = std::fs::File::open(&input).expect("open state-log");

    // ── Read header ──
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).expect("read magic");
    assert_eq!(&magic, b"HLOG", "not an HLOG file");

    // ── First pass: count records, find ranges ──
    // Records: u64 step | u64 min_pos | u64 max_pos = 24 bytes
    let mut record_buf = [0u8; 24];
    let mut global_min_pos: u64 = u64::MAX;
    let mut global_max_pos: u64 = 0;
    let mut min_steps: u64 = u64::MAX;
    let mut max_steps: u64 = 0;
    let mut num_records: u64 = 0;

    loop {
        match file.read_exact(&mut record_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("read error: {}", e),
        }
        let steps = u64::from_le_bytes(record_buf[0..8].try_into().unwrap());
        let min_pos = u64::from_le_bytes(record_buf[8..16].try_into().unwrap());
        let max_pos = u64::from_le_bytes(record_buf[16..24].try_into().unwrap());
        global_min_pos = global_min_pos.min(min_pos);
        global_max_pos = global_max_pos.max(max_pos);
        min_steps = min_steps.min(steps);
        max_steps = max_steps.max(steps);
        num_records += 1;
    }

    if num_records == 0 {
        eprintln!("No records found");
        return;
    }

    eprintln!(
        "{} records, steps [{}, {}], pos [{}, {}]",
        num_records, min_steps, max_steps, global_min_pos, global_max_pos
    );

    // ── Second pass: render into image ──
    let pos_range = (global_max_pos - global_min_pos + 1) as f64;
    let step_range = (max_steps - min_steps + 1) as f64;

    // Image buffer: RGB, black background
    let mut pixels = vec![0u8; width * height * 3];

    let mut file = std::fs::File::open(&input).expect("reopen");
    file.read_exact(&mut [0u8; 4]).unwrap(); // skip magic

    let color: [u8; 3] = [0x40, 0xc0, 0xff]; // cyan-ish

    loop {
        match file.read_exact(&mut record_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => panic!("read error: {}", e),
        }
        let steps = u64::from_le_bytes(record_buf[0..8].try_into().unwrap());
        let min_pos = u64::from_le_bytes(record_buf[8..16].try_into().unwrap());
        let max_pos = u64::from_le_bytes(record_buf[16..24].try_into().unwrap());

        let y = ((steps - min_steps) as f64 / step_range * height as f64) as usize;
        let y = y.min(height - 1);

        let x0 = ((min_pos - global_min_pos) as f64 / pos_range * width as f64) as usize;
        let x1 = ((max_pos - global_min_pos) as f64 / pos_range * width as f64) as usize;
        let x0 = x0.min(width - 1);
        let x1 = x1.min(width - 1);

        // Draw a horizontal line from x0 to x1 at row y
        for x in x0..=x1 {
            let idx = (y * width + x) * 3;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
        }
    }

    // ── Write PPM ──
    let mut out = std::io::BufWriter::new(std::fs::File::create(&output).expect("create output"));
    write!(out, "P6\n{} {}\n255\n", width, height).unwrap();
    out.write_all(&pixels).unwrap();
    drop(out);

    eprintln!("Wrote {}x{} PPM to {}", width, height, output);
}

fn get_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .map(|i| args[i + 1].clone())
}
