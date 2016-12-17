//! A motor controller interface (simulates the real one).

use std::iter::Iterator;

use simulation::{Simulation, SimData};

pub trait Controller: Sized {
    /// Returns the controller data at the current time.
    fn take_data(&self) -> ControllerData;
    /// Makes a single step in the algorithm.
    fn control_step(&mut self) -> ControllerData;

    fn control_until(&mut self, t_final: f64) -> ControlUntil<Self> {
        ControlUntil {
            controller: self,
            t_final: t_final,
        }
    }
}

pub struct ControlUntil<'a, T>
    where T: 'a + Controller
{
    controller: &'a mut T,
    t_final: f64,
}

pub struct ControllerData {
    pub sim_data: SimData,
    pub rate: f64,
}

impl<'a, T> Iterator for ControlUntil<'a, T>
    where T: 'a + Controller
{
    type Item = ControllerData;

    fn next(&mut self) -> Option<ControllerData> {
        if self.controller.take_data().sim_data.time > self.t_final {
            None
        } else {
            Some(self.controller.control_step())
        }
    }
}

/// The standard controller algorithm
pub struct StdController {
    sim: Simulation,
    last_rate: f64,
    // Move up = `true`
    direction: bool,
    step_size: f64,
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
}

impl Controller for StdController {
    fn take_data(&self) -> ControllerData {
        ControllerData {
            sim_data: self.sim.take_data(),
            rate: self.last_rate,
        }
    }

    fn control_step(&mut self) -> ControllerData {
        // Here is where we should implement the controller algorithm
        let sim = &mut self.sim;
        let d1 = sim.take_data();
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

        if rate < self.last_rate {
            // Switch directions and decrease step size
            self.direction = !self.direction;
            self.step_size *= 0.8;
            // Make sure it doesn't get too low
            if self.step_size < 0.001 {
                self.step_size = 0.001;
            }
        }

        // Move motor
        let step = if self.direction {
            self.step_size
        } else {
            -self.step_size
        };
        sim.set_freq(d3.frequency + step);

        // Give it time to settle
        for _ in sim.run_for(5.0) {}

        ControllerData {
            sim_data: d3,
            rate: rate,
        }
    }
}
