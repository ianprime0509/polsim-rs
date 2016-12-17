//! A motor controller interface (simulates the real one).

use std::iter::Iterator;

use simulation::{Simulation, SimData};

/// The standard controller algorithm
pub struct StdController {
    sim: Simulation,
    last_rate: f64,
    // Move up = `true`
    direction: bool,
    step_size: f64,
}

pub struct StdControlUntil<'a> {
    controller: &'a mut StdController,
    t_final: f64,
}

pub struct ControllerData {
    /// The last data point taken
    pub sim_data: SimData,
    pub rate: f64,
}

impl StdController {
    pub fn new(sim: Simulation) -> StdController {
        StdController {
            sim: sim,
            last_rate: 0.0,
            direction: true,
            step_size: 0.03,
        }
    }

    pub fn control_until(&mut self, t_final: f64) -> StdControlUntil {
        StdControlUntil {
            controller: self,
            t_final: t_final,
        }
    }
}

impl<'a> Iterator for StdControlUntil<'a> {
    type Item = ControllerData;

    fn next(&mut self) -> Option<ControllerData> {
        // Here is where we should implement the controller algorithm
        let sim = &mut self.controller.sim;
        let d1 = sim.take_data();
        if d1.time > self.t_final {
            return None;
        }
        for _ in sim.run_for(1.0) {}
        let d2 = sim.take_data();
        for _ in sim.run_for(1.0) {}
        let d3 = sim.take_data();

        // Calculate rate
        let p1 = (d2.pn + d1.pn) / 2.0;
        let p2 = (d3.pn + d2.pn) / 2.0;
        let e1 = (d2.time + d1.time) / 2.0;
        let e2 = (d3.time + d2.time) / 2.0;
        let rate = (p2 - p1) / (e2 - e1);

        if rate < self.controller.last_rate {
            // Switch directions and decrease step size
            self.controller.direction = !self.controller.direction;
            self.controller.step_size *= 0.8;
            // Make sure it doesn't get too low
            if self.controller.step_size < 0.001 {
                self.controller.step_size = 0.001;
            }
        }

        // Move motor
        let step = if self.controller.direction {
            self.controller.step_size
        } else {
            -self.controller.step_size
        };
        sim.set_freq(d3.frequency + step);

        // Give it time to settle
        for _ in sim.run_for(5.0) {}

        Some(ControllerData {
            sim_data: d3,
            rate: rate,
        })
    }
}
