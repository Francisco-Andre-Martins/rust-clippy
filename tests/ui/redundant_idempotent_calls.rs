//@check-pass
#![warn(clippy::redundant_idempotent_calls)]

fn nothing_ever_happens(){
    let var = 1.32_f64;  
}

fn should_not_trigger(){
    let var3 = 1.32_f64.floor();

}

fn two_calls_on_init(){
    let potato = 1.32_f64.floor().floor();
}

fn some_code_in_between(){
    let the_president = 420.69_f64.floor();
    let is_corrupt = false;
    the_president = the_president.floor();
}

fn if_in_between_no_trigger(){
    let apple = 3.14159_f64;
    apple = apple.floor();
    let delicious = true;
    if ( delicious){
        apple = 6.9;
    }
    apple.floor();
}
fn if_in_between_should_trigger(){
    let apple = 3.14159_f64;
    apple = apple.floor();
    let delicious = true;
    let plate_full = true;
    if ( delicious){
        plate_full = false;
    }
    apple.floor();
}
fn while_should_trigger(){
    let raspberry = 3.14159265_f64;
    raspberry.floor();
    let i = 1;
    while (i<10){
        i++;
    }
    raspberry.floor();
}
fn while_no_trigger(){
    let raspberry = 3.14159265_f64;
    raspberry.floor();
    let i = 1;
    while (i<10){
        i++;
        raspberry+= 0.5;
    }
    raspberry.floor();
}
fn main() {
    let var = 1.32_f64.floor().floor();
    let var2 = 1.32_f64;
    let var3 = 1.32_f64.floor();
    var3= var3.floor();
}
