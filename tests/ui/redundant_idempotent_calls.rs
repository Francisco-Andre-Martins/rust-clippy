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
fn complicated_cases(){
    let mut c = 1.32_f64.floor();
    let mut sometime_ago=false;
    if(sometime_ago){
        println!("sorry");
    } else{
        println!("too early for that");
    }
    while(sometime_ago){
        c = c.floor();
        //~^ redundant_idempotent_calls
        sometime_ago=false;
    }
}
fn complicated_casestoo(){
    let c = 1.32_f64.floor();
    let mut sometime_ago=false;
    if(sometime_ago){
        println!("sorry");
    } else{
        println!("too early for that");
    }
    while(sometime_ago){
        sometime_ago=false;
    }
}
fn simple_one(){
    let mut c =1.32_f64;
    c= c.floor();
    c = c.floor();
    //~^ redundant_idempotent_calls
}

fn false_positive_non_idempotent_pollutes_map() {
    let s = "Never gonna give you up".to_string();
    let _t = s.to_lowercase();
    s.to_lowercase();
}

fn closure_should_not_trigger() {
    let mut s = "Never gonna let you down".to_lowercase();
    let mut f = || s = String::from("world");
    f();
    s.to_lowercase();
}

fn main() {
    let var = 1.32_f64.floor().floor();
    //~^ redundant_idempotent_calls
    let var2 = 1.32_f64;
    let mut var3 = 1.32_f64.floor();
    var3 = var3.floor();
    //~^ redundant_idempotent_calls
}
