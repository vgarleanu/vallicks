#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(vallicks::test_runner)]
#![reexport_test_harness_main = "test_main"]

use naked_std::thread;
use vallicks::prelude::*;

struct Test(pub u32);

fn n10() {
    let test = Test(123);
    panic!();
}
fn n9() {
    n10();
}
fn n8() {
    n9();
}
fn n7() {
    n8();
}
fn n6() {
    n7();
}
fn n5() {
    n6();
}
fn n4() {
    n6();
}

fn n3() {
    n4();
}

fn n2() {
    n3();
}

fn n1() {
    n2();
}

fn n() {
    n1();
}

#[entrypoint]
fn main() {
    let t = thread::spawn(move || {
        n();
    });

    t.join();
    println!("Ok");
}
