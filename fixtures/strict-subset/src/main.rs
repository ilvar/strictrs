mod names {
    pub const VALUE: u32 = 7;
}

use names::*;

static mut COUNT: u32 = 0;

enum Mode {
    Fast,
    Slow,
}

pub fn public_value() {
    let values = [VALUE];
    let first = values.get(0).unwrap();
    let narrowed = *first as u8;
    let mode = Mode::Fast;

    match mode {
        Mode::Fast => println!("{narrowed}"),
        _ => println!("slow"),
    }

    let _text = std::fs::read_to_string("missing.txt");
}

// strictrs: capability
mod process_boundary {
    pub fn status() {
        let _ = std::process::Command::new("true").status();
    }
}

fn main() {
    public_value();
    process_boundary::status();
    let _ = Mode::Slow;
}
