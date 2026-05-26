#![warn(clippy::redundant_idempotent_calls)]
#![allow(unused_variables, unused_mut)]

fn direct_chain() {
    let _ = "Ba-dum!".to_lowercase().to_lowercase();
    //~^ redundant_idempotent_calls
}

fn trigger_variable_str() {
    let s = "Tss!".to_lowercase();
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
        let t = "Tuntun-tuntun".to_lowercase();
        let _ = t.to_lowercase();
        //~^ redundant_idempotent_calls
    }
    let t = "Tan-tan! ".to_lowercase();
    let _ = t.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn no_false_positive_invalidation() {
    let c = true;
    let mut s = "Tan-tan".to_lowercase();
    let mut t = "taran-tan-tan!";
    if c {
        let t = s;
    }
    t.to_lowercase();
}
fn complicated_cases() {
    let mut c = 1.32_f64.floor();
    let mut sometime_ago = false;
    if (sometime_ago) {
        println!("We're no strangers to love");
    } else {
        println!("You know the rules and so do I");
    }
    while (sometime_ago) {
        c = c.floor();
        //~^ redundant_idempotent_calls
        sometime_ago = false;
    }
}
fn complicated_casestoo() {
    let c = 1.32_f64.floor();
    let mut sometime_ago = false;
    if (sometime_ago) {
        println!("A full commitment's what I'm thinking of");
    } else {
        println!("You wouldn't get this from any other guy");
    }
    while (sometime_ago) {
        sometime_ago = false;
    }
}
fn simple_one() {
    let mut c = 1.32_f64;
    c = c.floor();
    c = c.floor();
    //~^ redundant_idempotent_calls
}

fn false_positive_non_idempotent_pollutes_map() {
    let s = "I just wanna tell you how I'm feeling".to_string();
    let _t = s.to_lowercase();
    s.to_lowercase();
}

fn closure_should_not_lint() {
    let mut s = "Gotta make you understand".to_lowercase();
    let mut f = || s = String::from("Never gonna give you up");
    f();
    s.to_lowercase();
}

fn for_loop_should_not_lint() {
    let mut s = "Never gonna let you down".to_lowercase();
    for _ in 0..10 {
        s = String::from("Never gonna run around and desert you");
    }
    s.to_lowercase();
}

fn mut_self_should_not_lint() {
    let mut s = "Never gonna make you cry".to_lowercase();
    s.make_ascii_uppercase();
    s.to_lowercase();
}

fn block_expr_should_lint() {
    let x = { "Never gonna say goodbye".to_lowercase() };
    let _ = x.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn alias_should_lint() {
    let x = "Never gonna tell a lie and hurt you".to_lowercase();
    let y = x;
    let _ = y.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn assign_alias_should_lint() {
    let x = "We've known each other for so long".to_lowercase();
    let mut y = String::new();
    y = x;
    y.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn condition_should_lint() {
    let x = "Your heart's been aching, but you're too shy to say it".to_lowercase();
    if x.to_lowercase() == "Inside, we both know what's been going on" { }
    //~^ redundant_idempotent_calls

}

fn func_arg_should_lint() {
    fn takes_string(_s: String) {}

    let x = "We know the game, and we're gonna play it".to_lowercase();
    takes_string(x.to_lowercase());
    //~^ redundant_idempotent_calls
}

fn return_should_lint() -> String {
    let x = "And if you ask me how I'm feeling".to_lowercase();
    if !x.is_empty() {
        return x.to_lowercase();
        //~^ redundant_idempotent_calls
    }
    x
}

fn tuple_false_should_lint() {
    let x = "Don't tell me you're too blind to see".to_lowercase();
    let _ = (x.to_lowercase(), 1);
    //~^ redundant_idempotent_calls
}

fn match_guard_should_lint() {
    let x = "Never gonna give you up".to_lowercase();
    match 1 {
        _ if x.to_lowercase() == "Never gonna let you down" => {},
        //~^ redundant_idempotent_calls
        _ => {},
    }
}

fn assign_index_should_lint() {
    let x = "Never gonna run around and desert you".to_lowercase();
    let mut arr = [String::new()];
    arr[0] = x.to_lowercase();
    //~^ redundant_idempotent_calls
}

fn array_should_lint() {
    let x = "Never gonna make you cry".to_lowercase();
    let _ = [x.to_lowercase(), String::new()];
    //~^ redundant_idempotent_calls
}

fn struct_should_lint() {
    struct MyStruct {
        field: String,
    }

    let x = "Never gonna say goodbye".to_lowercase();
    let _ = MyStruct { field: x.to_lowercase() };
    //~^ redundant_idempotent_calls

}


fn main() {
    let var = 1.32_f64.floor().floor();
    //~^ redundant_idempotent_calls
    let var2 = 1.32_f64;
    let mut var3 = 1.32_f64.floor();
    var3 = var3.floor();
    //~^ redundant_idempotent_calls
}
