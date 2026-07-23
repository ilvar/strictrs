enum Direction {
    North,
    South,
}

fn type_mismatch() {
    let _: u32 = "not a number";
}

fn unknown_name() {
    missing_function();
}

fn misspelled_method() {
    let values = vec![1, 2, 3];
    let _ = values.frist();
}

fn non_exhaustive_match(direction: Direction) {
    match direction {
        Direction::North => {}
    }
}

fn main() {
    type_mismatch();
    unknown_name();
    misspelled_method();
    non_exhaustive_match(Direction::South);
}
