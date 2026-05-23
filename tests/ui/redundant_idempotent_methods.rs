#![warn(clippy::redundant_idempotent_methods)]

fn main() {
    let x = 3.14_f32.floor();
    x = x.floor(); 
}
