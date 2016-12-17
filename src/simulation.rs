//! This is the simulation, ported from the C++ ROOT version.

#![allow(dead_code)]

use std::f64::consts;
use std::iter::Iterator;

use rand;

/// Elementary charge (in C)
pub static ELEM_CHARGE: f64 = 1.602176662e-19;

/// Parameters for alpha/beta vs frequency fit
static FIT_A: f64 = 0.545266;
static FIT_S: f64 = 0.088415;

/// The m1 and m2 parameters are the centers of the beta/alpha
/// distributions, respectively. The actual center changes
/// as dose is added, according to:
/// m1 = (FIT_M1_BASE - FIT_M1_COEFF) + FIT_M1_COEFF * exp(FIT_M1_RATE * dose)
/// and similarly for m2.
/// The justification for this model can be found in the 2016 presentation,
/// which itself got these parameters from the SANE frequency vs dose data.
/// The FIT_M1_BASE and FIT_M2_BASE parameters are the centers of the distributions
/// when polarizing with the beam off (in GHz).
static FIT_M1_BASE: f64 = 140.286;
static FIT_M1_COEFF: f64 = 0.045;
static FIT_M1_RATE: f64 = -0.38e-15;
static FIT_M2_BASE: f64 = 140.468;
static FIT_M2_COEFF: f64 = -0.065;
static FIT_M2_RATE: f64 = -3.8e-15;

/// Simulation parameters
static TIME_STEP: f64 = 1.0;
static N_ITER: i32 = 1000;

/// Randomness parameters:
/// There are fluctuations caused by thermal effects which will manifest as
/// a fraction of the polarization value, but there are also fluctuations
/// which will manifest uniformly at any polarization (even at 0);
/// these are probably measurement uncertainties
static THERMAL_RANDOMNESS: f64 = 0.02;
static BASE_RANDOMNESS: f64 = 0.002;

/// Not sure what kind of a parameter this is
/// Whenever the beam is on, C is increased according to
/// (delta)C = IRRADIATION_FACTOR * beam_current * (delta)t
static IRRADIATION_FACTOR: f64 = 1e-10;

/// Represents a solid polarized target experiment.
pub struct Simulation {
    /// Physical constants
    t1n: f64,
    t1e: f64,

    /// "External" physical parameters
    /// The system_temperature is the temperature of the system;
    /// in normal situations, this is 1K (the temperature of the fridge)
    t: f64,
    freq: f64,
    temperature: f64,
    system_temperature: f64,

    /// Internal physical parameters
    alpha: f64,
    beta: f64,
    c: f64,
    pe0: f64,
    phi: f64,

    /// Polarization values
    /// pn_raw is the "raw polarization" (without noise)
    pn_raw: f64,
    pn: f64,
    pe: f64,

    /// Dose
    dose: f64,
    beam_current: f64,
}

/// For building `Simulation`s using the builder pattern
pub struct SimBuilder {
    freq: f64,
    pn: f64,
    pe: f64,
    c: f64,
    temperature: f64,
    t1n: f64,
    t1e: f64,
}

/// A single "data point" containing all observable data at a particular time
pub struct SimData {
    pub time: f64,
    pub pn: f64,
    pub pe: f64,
    pub frequency: f64,
    pub c: f64,
    pub temperature: f64,
    pub dose: f64,
}

/// An iterator returning data points for each time step
pub struct RunUntil<'a> {
    sim: &'a mut Simulation,
    t_final: f64,
}

impl SimBuilder {
    pub fn new(freq: f64) -> SimBuilder {
        SimBuilder {
            freq: freq,
            pn: 0.0,
            pe: -1.0,
            c: 0.000136073,
            temperature: 1.0,
            t1n: 25.0 * 60.0,
            t1e: 0.03,
        }
    }

    pub fn initial_pol(&mut self, t1n: f64, t1e: f64) -> &mut SimBuilder {
        self.t1n = t1n;
        self.t1e = t1e;
        self
    }

    pub fn c(&mut self, c: f64) -> &mut SimBuilder {
        self.c = c;
        self
    }

    pub fn temperature(&mut self, temperature: f64) -> &mut SimBuilder {
        self.temperature = temperature;
        self
    }

    pub fn physical_constants(&mut self, t1n: f64, t1e: f64) -> &mut SimBuilder {
        self.t1n = t1n;
        self.t1e = t1e;
        self
    }

    pub fn build(&self) -> Simulation {
        let mut sim = Simulation {
            t1n: self.t1n,
            t1e: self.t1e,

            t: 0.0,
            freq: self.freq,
            temperature: self.temperature,
            system_temperature: self.temperature,

            alpha: 0.0,
            beta: 0.0,
            c: self.c,
            pe0: 0.0,
            phi: 0.0,

            pn_raw: self.pn,
            pn: self.pn,
            pe: self.pe,

            dose: 0.0,
            beam_current: 0.0,
        };

        // Make sure we do all the necessary initialization
        sim.set_freq(self.freq);
        sim.set_temperature(self.temperature);
        sim.beam_off();

        sim
    }
}

impl Simulation {
    pub fn set_freq(&mut self, freq: f64) {
        self.freq = freq;
        self.calc_transition_rates();
    }

    pub fn set_system_temperature(&mut self, temperature: f64) {
        self.system_temperature = temperature;
    }

    pub fn beam_on(&mut self, current: f64) {
        self.beam_current = current;
    }

    pub fn beam_off(&mut self) {
        self.beam_current = 0.0;
    }

    pub fn run_until(&mut self, t_final: f64) -> RunUntil {
        RunUntil {
            sim: self,
            t_final: t_final,
        }
    }

    pub fn run_for(&mut self, time: f64) -> RunUntil {
        let t = self.t;
        self.run_until(t + time)
    }

    pub fn anneal(&mut self, time: f64, temperature: f64) {
        // Reset phi (i.e. remove negative effects of irradiation)
        self.phi = 0.0;
        // Maybe change t1n?
        self.t1n *= 0.8;

        let temp_tmp = self.system_temperature;
        self.set_system_temperature(temperature);
        let t = self.t;
        self.run_until(t + time);
        self.set_system_temperature(temp_tmp);
    }

    pub fn take_data(&self) -> SimData {
        SimData {
            time: self.t,
            pn: self.pn,
            pe: self.pe,
            frequency: self.freq,
            c: self.c,
            temperature: self.temperature,
            dose: self.dose,
        }
    }

    fn set_temperature(&mut self, temperature: f64) {
        self.pe0 = -(2.0 / temperature).tanh();
        self.temperature = temperature;
    }

    fn time_step(&mut self) {
        // Parameters for temperature change (exponential growth/decay)
        // TEMP_SS = steady-state temperature
        // K_TEMP = rate of exponential increase
        // If we're annealing, we shouldn't allow the temperature to change
        // (assume anneals occur at constant temperature)
        let k_temp = 0.01;
        let temp_ss = self.system_temperature + self.beam_current / 100.0;

        // Increase phi according to some exponential growth when the beam is on
        // Parameters are similar to those for temperature change
        let k_phi = self.beam_current / 1e7;
        let phi_ss = 0.001;

        for _ in 0..N_ITER {
            // Calculate constants (for convenience)
            let a_const = -self.t1e / self.t1n - (self.c / 2.0) * (self.alpha + self.beta) -
                          self.phi;
            let b_const = (self.c / 2.0) * (self.alpha - self.beta);
            let c_const = (self.alpha - self.beta) / 2.0;
            let d_const = -1.0 - (self.alpha + self.beta) / 2.0;

            // Calculate rates
            let pn_prime = (a_const * self.pn_raw + b_const * self.pe) / self.t1e;
            let pe_prime = (c_const * self.pn_raw + d_const * self.pe + self.pe0) / self.t1e;

            // Shortcut calculation
            let time_amt = TIME_STEP / N_ITER as f64;

            // Update pn and pe (Euler's method)
            self.pn_raw += pn_prime * time_amt;
            self.pe += pe_prime * time_amt;
            // Update temperature and phi
            let temp = self.temperature;
            self.set_temperature(temp + time_amt * k_temp * (temp_ss - temp));
            self.phi += time_amt * k_phi * (phi_ss - self.phi);

            // Update C and dose
            self.c += IRRADIATION_FACTOR * self.beam_current * time_amt;
            self.dose += (self.beam_current * 1e-9 / ELEM_CHARGE) * time_amt;

            // Calculate new transition rates (alpha and beta)
            self.calc_transition_rates();

            // Update time
            self.t += time_amt;
        }

        // Update "noisy pn"
        self.pn = self.pn_noisy();
    }

    fn calc_transition_rates(&mut self) {
        // Calculate distribution parameters (the means m1 and m2 are particularly important)
        let fit_m1 = (FIT_M1_BASE - FIT_M1_COEFF) + FIT_M1_COEFF * (FIT_M1_RATE * self.dose).exp();
        let fit_m2 = (FIT_M2_BASE - FIT_M2_COEFF) + FIT_M2_COEFF * (FIT_M2_RATE * self.dose).exp();
        let scale = FIT_A / ((2.0 * consts::PI).sqrt() * FIT_S);
        let diff1 = self.freq - fit_m1;
        let diff2 = self.freq - fit_m2;
        let exp1 = (-diff1 * diff1 / (2.0 * FIT_S * FIT_S)).exp();
        let exp2 = (-diff2 * diff2 / (2.0 * FIT_S * FIT_S)).exp();

        self.alpha = scale * exp2;
        self.beta = scale * exp1;
    }

    fn pn_noisy(&mut self) -> f64 {
        let thermal_noise = THERMAL_RANDOMNESS * (0.5 - rand::random::<f64>());
        let uniform_noise = BASE_RANDOMNESS * (0.5 - rand::random::<f64>());

        self.pn_raw * (1.0 + thermal_noise) + uniform_noise
    }
}

impl<'a> Iterator for RunUntil<'a> {
    type Item = SimData;

    fn next(&mut self) -> Option<SimData> {
        if self.sim.t < self.t_final {
            let data = self.sim.take_data();
            self.sim.time_step();
            Some(data)
        } else {
            None
        }
    }
}
