extern crate rand;

mod simulation;

use std::time::Instant;

use simulation::SimBuilder;

fn main() {
    let mut sim = SimBuilder::new(140.15).build();

    let start = Instant::now();
    for d in sim.run_until(10000.0) {
        // println!("{:.2} {:.6}", d.time, d.pn);
    }
    let elapsed = start.elapsed();
    let bench = (elapsed.as_secs() as f64 * 1000.0) + (elapsed.subsec_nanos() as f64 / 1e6);
    println!("Time: {} ms", bench);
}
