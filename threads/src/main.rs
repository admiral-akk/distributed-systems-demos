use std::{thread, time::Duration};

use rand::Rng;
fn main() {
    let mut v = Vec::new();
    for i in 0..10 {
        let s = rand::thread_rng().gen_range(1..1000);
        v.push(thread::spawn(move || {
            thread::sleep(Duration::from_millis(s));
            println!("Hello from thread {}, waited: {} ms", i, s);
        }));
    }
    for t in v {
        t.join().unwrap();
    }
    println!("Hello, world!");
}
