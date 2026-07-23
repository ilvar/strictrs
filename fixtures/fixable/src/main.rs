mod first_values {
    pub const FIRST: u32 = 1;
}

mod last_values {
    pub const LAST: u32 = 3;
}

use first_values::*;
use last_values::*;

fn main() {
    println!("{} {}", FIRST, LAST);
}
