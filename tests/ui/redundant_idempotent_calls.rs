//@check-pass
#![warn(clippy::redundant_idempotent_calls)]

fn main() {
    let var = 1.32_f64.floor().floor();
}
