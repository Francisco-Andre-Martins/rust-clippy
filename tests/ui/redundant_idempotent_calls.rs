#![warn(clippy::redundant_idempotent_calls)]
#![allow(unused_variables, unused_mut)]

fn direct_chain() {
    let _ = "HELLO".to_lowercase().to_lowercase();
    //~^ redundant_idempotent_calls
}

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

fn no_false_positive_variable_leak() {
    let c = true;
    if c {
        let t = "HELLO".to_lowercase();
        let _ = t.to_lowercase();
        //~^ redundant_idempotent_calls
    }
    let t = "WORLD".to_lowercase();
    let _ = t.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn no_false_positive_invalidation() {
    let c = true;
    let mut s = "HELLO".to_lowercase();
    let mut t = "WORLD";
    if c {
        let t = s;
    }
    t.to_lowercase();
}

fn main() {
    let var = 1.32_f64.floor().floor();
    let var2 = 1.32_f64;
    let mut var3 = 1.32_f64.floor();
    var3 = var3.floor();
    //~^ redundant_idempotent_calls
}
