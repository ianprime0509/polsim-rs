//! Simulates the PDP software (performing sweeps to collect polarization data).

use rand;

use simulation::{Data, Simulation};

const MS_PER_SWEEP: f64 = 64.0;
/// The fractional error in polarization per sweep.
const SWEEP_UNCERTAINTY: f64 = 0.04;

/// The state of the PDP simulation.
pub struct Pdp {
    /// The underlying system.
    sim: Simulation,
    /// The number of sweeps per data reading.
    n_sweeps: u32,
}

/// An iterator returning data points for each time step.
pub struct RunUntil<'a> {
    pdp: &'a mut Pdp,
    t_final: f64,
}

impl Pdp {
    /// Create a new PDP simulator with the given underlying `Simulation` and number of sweeps.
    pub fn new(sim: Simulation, n_sweeps: u32) -> Self {
        Pdp { sim, n_sweeps }
    }

    /// Gets a single data point by sweeping.
    pub fn take_data(&mut self) -> Data {
        // Collect polarization for averaging
        let mut pn = 0.0;
        for _ in 0..self.n_sweeps {
            pn += self.sweep().pn;
        }
        pn /= self.n_sweeps as f64;

        // Get a data point and change its polarization to the "fuzzed" value.
        let mut data = self.sim.take_data();
        data.pn = pn;
        data
    }

    /// Runs sweeps until the specified time, using an iterator to return data points.
    pub fn run_until_iter(&mut self, t_final: f64) -> RunUntil {
        RunUntil { pdp: self, t_final }
    }

    pub fn run_for_iter(&mut self, time: f64) -> RunUntil {
        let t = self.sim.take_data().time;
        self.run_until_iter(t + time)
    }

    /// Perform a single sweep and return its data.
    fn sweep(&mut self) -> Data {
        self.sim.run_for(MS_PER_SWEEP / 1000.0, 0.001);
        let mut sweep_data = self.sim.take_data();
        // Fuzz polarization
        sweep_data.pn += SWEEP_UNCERTAINTY * (rand::random::<f64>() - 0.5);
        sweep_data
    }
}

impl<'a> Iterator for RunUntil<'a> {
    type Item = Data;

    fn next(&mut self) -> Option<Data> {
        if self.pdp.sim.take_data().time < self.t_final {
            Some(self.pdp.take_data())
        } else {
            None
        }
    }
}

