//! A motor controller interface (simulates the real one).

use std::iter::Iterator;
use std::marker::PhantomData;

use rand;

use simulation::{Simulation, SimData};

pub trait Controller: Sized {
    /// Creates a new controller for the given simulation
    fn from_sim(sim: Simulation) -> Self;
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

/// This seems like an abuse of the type system but whatever
pub trait ThreePointControllerFn {
    /// Calculate the rate from three data points.
    fn calc_rate(d1: &SimData, d2: &SimData, d3: &SimData) -> f64;
}

/// A generic controller that runs based on a rate calculated from three
/// data points (the "rate" doesn't actually have to be a rate, but it
/// will be compared in the same way).
pub struct ThreePointController<T: ThreePointControllerFn> {
    sim: Simulation,
    last_rate: f64,
    // Move up = `true`
    direction: bool,
    step_size: f64,
    /// This is definitely a type system hack
    wat: PhantomData<T>,
}

impl<T> Controller for ThreePointController<T>
    where T: ThreePointControllerFn
{
    fn from_sim(sim: Simulation) -> ThreePointController<T> {
        ThreePointController {
            sim: sim,
            last_rate: 0.0,
            direction: true,
            step_size: 0.03,
            wat: PhantomData,
        }
    }

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
        let rate = T::calc_rate(&d1, &d2, &d3);

        if rate < self.last_rate {
            // Switch directions and decrease step size
            self.direction = !self.direction;
            self.step_size *= 0.8;
            // Make sure it doesn't get too low
            if self.step_size < 0.001 {
                self.step_size = 0.001;
            }
        }

        // Update last rate
        self.last_rate = rate;

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

/// The standard controller algorithm
pub struct StdControllerFn;

impl ThreePointControllerFn for StdControllerFn {
    fn calc_rate(d1: &SimData, d2: &SimData, d3: &SimData) -> f64 {
        let p1 = (d2.pn + d1.pn) / 2.0;
        let p2 = (d3.pn + d2.pn) / 2.0;
        let e1 = (d2.time + d1.time) / 2.0;
        let e2 = (d3.time + d2.time) / 2.0;
        (p2 - p1) / (e2 - e1)
    }
}

pub type StdController = ThreePointController<StdControllerFn>;

/// The standard controller algorithm (variant)
pub struct StdControllerFn2;

impl ThreePointControllerFn for StdControllerFn2 {
    fn calc_rate(d1: &SimData, d2: &SimData, d3: &SimData) -> f64 {
        let r1 = (d2.pn - d1.pn) / (d2.time - d1.time);
        let r2 = (d3.pn - d2.pn) / (d3.time - d2.time);
        (r1 + r2) / 2.0
    }
}

pub type StdController2 = ThreePointController<StdControllerFn2>;

/// Test of a "k-val based" controller algorithm
pub struct KValControllerFn;

impl ThreePointControllerFn for KValControllerFn {
    fn calc_rate(d1: &SimData, d2: &SimData, d3: &SimData) -> f64 {
        // The "rate" here is actually going to be the steady state value
        // For convenience
        let (x1, y1) = (d1.time, d1.pn);
        let (x2, y2) = (d2.time, d2.pn);
        let (x3, y3) = (d3.time, d3.pn);

        // Calculate the Vandermonde determinant
        let det = (x1 - x3) * (x2 - x3) * (x1 - x2);
        // Essentially, we want to fit a polynomial
        // ax^2 + bx + c to these three data points
        // so that we can compute the derivative
        let a = (y1 * (x2 - x3) + y2 * (x3 - x1) + y3 * (x1 - x2)) / det;
        let b = (y1 * (x3 * x3 - x2 * x2) + y2 * (x1 * x1 - x3 * x3) + y3 * (x2 * x2 - x1 * x1)) /
                det;
        // Nobody cares what c is

        // Try to compute first and second derivatives at x2
        let first_deriv = 2.0 * a * x2 + b;
        let second_deriv = 2.0 * a;

        // Now return the steady state
        let ss = y2 - first_deriv * first_deriv / second_deriv;
        ss
    }
}

pub type KValController = ThreePointController<KValControllerFn>;

/// A random controller (control group)
pub struct RandController {
    sim: Simulation,
    last_rate: f64,
    direction: bool,
    step_size: f64,
}

impl Controller for RandController {
    fn from_sim(sim: Simulation) -> RandController {
        RandController {
            sim: sim,
            last_rate: 0.0,
            direction: true,
            step_size: 0.03,
        }
    }

    fn take_data(&self) -> ControllerData {
        ControllerData {
            sim_data: self.sim.take_data(),
            rate: self.last_rate,
        }
    }

    fn control_step(&mut self) -> ControllerData {
        // Here is where we should implement the controller algorithm
        let sim = &mut self.sim;
        for _ in sim.run_for(1.0) {}
        for _ in sim.run_for(1.0) {}
        let d3 = sim.take_data();

        if rand::random() {
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
            rate: 0.0,
        }
    }
}
