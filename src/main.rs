#[macro_use]
extern crate error_chain;
extern crate rand;

mod errors {
    error_chain!{}
}
use errors::*;

mod pdp;
mod simulation;

use std::fs::File;
use std::io::{self, Write};

use pdp::Pdp;
use simulation::Builder;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    println!("Welcome to polsim v{}.", VERSION);

    loop {
        match run() {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => {
                eprintln!("Error: {}", e.iter().nth(0).unwrap());

                for cause in e.iter().skip(1) {
                    eprintln!("Caused by: {}", cause);
                }

                if let Some(backtrace) = e.backtrace() {
                    eprintln!("Backtrace: {:?}", backtrace);
                }
                std::process::exit(1);
            }
        }
    }
}

fn run() -> Result<bool> {
    println!();
    match prompt("Choose operation:")?.as_str() {
        "pdp" => pdp()?,
        "sim" => sim()?,
        "help" => {
            println!("Available commands are:
---- pdp: Simulate the PDP environment
---- sim: Run the simulation without sweeping")
        }
        "quit" => return Ok(true),
        s => {
            println!("Unknown command `{}`. Type `help` for available commands.",
                     s)
        }
    }

    Ok(false)
}

fn pdp() -> Result<()> {
    let mut output: Box<Write> = match prompt("Output file name (leave blank for stdout):")?
              .as_str() {
        "" => Box::new(io::stdout()),
        s => {
            Box::new(File::create(s)
                         .chain_err(|| format!("could not create output file `{}`", s))?)
        }
    };
    let freq = prompt("Frequency (GHz):")?
        .parse::<f64>()
        .chain_err(|| "invalid frequency")?;
    let time = prompt("Time to run (s):")?
        .parse::<f64>()
        .chain_err(|| "invalid time")?;
    let n_sweeps = prompt("Number of sweeps:")?
        .parse::<u32>()
        .chain_err(|| "invalid number of sweeps")?;

    let mut pdp = Pdp::new(Builder::new(freq).build(), n_sweeps);
    for data in pdp.run_for_iter(time) {
        writeln!(output, "{}", data.to_csv())
            .chain_err(|| "could not write to output file")?;
    }

    Ok(())
}

fn sim() -> Result<()> {
    let mut output: Box<Write> = match prompt("Output file name (leave blank for stdout):")?
              .as_str() {
        "" => Box::new(io::stdout()),
        s => {
            Box::new(File::create(s)
                         .chain_err(|| format!("could not create output file `{}`", s))?)
        }
    };
    let freq = prompt("Frequency (GHz):")?
        .parse::<f64>()
        .chain_err(|| "invalid frequency")?;
    let time = prompt("Time to run (s):")?
        .parse::<f64>()
        .chain_err(|| "invalid time")?;

    let mut sim = Builder::new(freq).build();
    for data in sim.run_for_iter(time, 0.001, 1.0) {
        writeln!(output, "{}", data.to_csv())
            .chain_err(|| "could not write to output file")?;
    }

    Ok(())
}

fn prompt(title: &str) -> Result<String> {
    print!("{} ", title);
    io::stdout()
        .flush()
        .chain_err(|| "could not flush prompt to stdout")?;

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .chain_err(|| "could not read input from stdin")?;

    Ok(buf.trim().into())
}

