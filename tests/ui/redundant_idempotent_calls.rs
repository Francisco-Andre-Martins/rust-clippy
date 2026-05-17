#![warn(clippy::redundant_idempotent_calls)]

fn main() {
    let var = 1.32.floor().floor();
}
