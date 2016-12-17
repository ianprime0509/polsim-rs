extern crate rand;

mod controller;
mod simulation;

use std::time::Instant;

use controller::StdController;
use simulation::SimBuilder;

fn main() {
    let sim = SimBuilder::new(140.15).build();
    let mut controller = StdController::new(sim);

    let start = Instant::now();
    for d in controller.control_until(300.0) {
        println!("{:.2} {:.6} {:.3} {:.6}",
                 d.sim_data.time,
                 d.sim_data.pn,
                 d.sim_data.frequency,
                 d.rate);
    }
    let elapsed = start.elapsed();
    let bench = (elapsed.as_secs() as f64 * 1000.0) + (elapsed.subsec_nanos() as f64 / 1e6);
    println!("Time: {} ms", bench);
}
