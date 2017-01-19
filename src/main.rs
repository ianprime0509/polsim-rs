extern crate rand;

mod controller;
mod simulation;

use std::collections::HashMap;
use std::thread;
use std::time::Instant;

use controller::{Controller, StdController, StdController2, RandController, KValController};
use simulation::SimBuilder;

fn main() {
    let n_trials = 100;
    let init_freq = 140.15;
    let min_freq = 140.17;
    let max_freq = 140.25;
    let time = 300.0;

    let start = Instant::now();
    // Do tests for each controller in a different thread
    let mut tests = HashMap::new();
    tests.insert("StdController",
                 thread::spawn(move || {
                     test_complete::<StdController>(n_trials, init_freq, min_freq, max_freq, time)
                 }));
    tests.insert("StdController2",
                 thread::spawn(move || {
                     test_complete::<StdController2>(n_trials, init_freq, min_freq, max_freq, time)
                 }));
    tests.insert("RandController",
                 thread::spawn(move || {
                     test_complete::<RandController>(n_trials, init_freq, min_freq, max_freq, time)
                 }));
    tests.insert("KValController",
                 thread::spawn(move || {
                     test_complete::<KValController>(n_trials, init_freq, min_freq, max_freq, time)
                 }));

    // Get results
    for (name, test) in tests {
        match test.join() {
            Ok((n, freq)) => {
                println!("{}: success {} / {} = {}; avg. freq = {}",
                         name,
                         n,
                         n_trials,
                         n as f64 / n_trials as f64,
                         freq)
            }
            Err(_) => println!("{}: panicked", name),
        }
    }

    let elapsed = start.elapsed();
    let bench = (elapsed.as_secs() as f64 * 1000.0) + (elapsed.subsec_nanos() as f64 / 1e6);
    println!("Time: {} ms", bench);
}

/// A more complete test, with multiple trials. Returns the number of successful trials and the
/// average frequency.
fn test_complete<T: Controller>(n_trials: i32,
                                init_freq: f64,
                                min_freq: f64,
                                max_freq: f64,
                                time: f64)
                                -> (i32, f64) {
    let mut n_success = 0;
    let mut sum = 0.0;

    for _ in 0..n_trials {
        let sim = SimBuilder::new(init_freq).build();
        let mut cont = T::from_sim(sim);
        for _ in cont.control_until(time) {}
        let d = cont.take_data();
        sum += d.sim_data.frequency;

        if min_freq <= d.sim_data.frequency && max_freq >= d.sim_data.frequency {
            n_success += 1;
        }
    }

    (n_success, sum / n_trials as f64)
}
