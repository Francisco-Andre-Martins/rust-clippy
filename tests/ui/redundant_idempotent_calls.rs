#![warn(clippy::redundant_idempotent_calls)]
#![allow(unused_variables, unused_mut)]

fn trigger_variable_str() {
    let s = "HELLO".to_lowercase();
    s.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn trigger_variable_float() {
    let x = 1.32_f64.floor();
    x.floor();
    //~^ redundant_idempotent_calls
}

fn main() {
    let var = 1.32_f64.floor().floor();
    let var2 = 1.32_f64;
    let mut var3 = 1.32_f64.floor();
    var3 = var3.floor();
}
