#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::unwrap_used
    )
)]

fn main() {
    println!("Hello from hello-strictrs.");
}
