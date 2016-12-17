extern crate rand;

mod controller;
mod simulation;

use std::time::Instant;

use controller::{Controller, StdController};
use simulation::SimBuilder;

fn main() {
    let n_trials = 100;
    let mut n_success = 0;

    let start = Instant::now();
    for _ in 0..n_trials {
        let sim = SimBuilder::new(140.15).build();
        let cont = StdController::new(sim);
        if test_controller(140.17, 140.27, 300.0, cont) {
            n_success += 1;
        }
    }
    let elapsed = start.elapsed();
    let bench = (elapsed.as_secs() as f64 * 1000.0) + (elapsed.subsec_nanos() as f64 / 1e6);
    println!("Time: {} ms", bench);
    println!("Success: {}/{} = {}",
             n_success,
             n_trials,
             n_success as f64 / n_trials as f64);
}

/// Tests a controller, to see if after `time` it'll end up in the given frequency range.
fn test_controller<T: Controller>(min_freq: f64, max_freq: f64, time: f64, mut cont: T) -> bool {
    for _ in cont.control_until(time) {}
    let d = cont.take_data();

    min_freq <= d.sim_data.frequency && max_freq >= d.sim_data.frequency
}
