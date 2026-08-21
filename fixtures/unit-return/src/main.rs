//! A public function that genuinely returns nothing.
//!
//! `strictrs::explicit_return_type` requires the `-> ()`; `clippy::unused_unit`
//! objects to it. Both firing would leave this function unwritable, so the
//! subset's rule wins and the clippy lint is not reported.

pub fn announce(message: &str) -> () {
    println!("{message}");
}

fn main() {
    announce("ready");
}
