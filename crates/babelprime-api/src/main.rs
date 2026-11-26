//! BabelPrime CLI.

use babelprime_core::{factor_to_json, primality_to_json, verify_json, VERSION};
use babelprime_factor::FactorJobConfig;
use clap::{Parser, Subcommand};
use std::io::{self, Read};
use std::process;

#[derive(Parser)]
#[command(name = "babelprime", version = VERSION, about = "Arbitrary-precision factorization and primality engine")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Factor an integer (decimal or 0x hex).
    Factor {
        input: String,
        #[arg(long, default_value_t = 0xB4B31)]
        seed: u64,
        #[arg(long, default_value_t = 30_000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = 10_000_000)]
        budget: u64,
        #[arg(long, default_value_t = 16)]
        mr_rounds: u32,
        #[arg(long, default_value_t = true)]
        ecm: bool,
        #[arg(long, default_value_t = 2000)]
        ecm_b1: u64,
        #[arg(long, default_value_t = 20)]
        ecm_curves: u32,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    /// Test primality (Miller–Rabin).
    Primality {
        input: String,
        #[arg(long, default_value_t = 16)]
        rounds: u32,
        #[arg(long, default_value_t = 0xB4B31)]
        seed: u64,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    /// Verify a factorization JSON result (stdin or file).
    Verify {
        #[arg(long)]
        file: Option<String>,
    },
    /// Print version and capability summary.
    Info,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Factor {
            input,
            seed,
            timeout_ms,
            budget,
            mr_rounds,
            ecm,
            ecm_b1,
            ecm_curves,
            json,
        } => {
            let config = FactorJobConfig {
                seed,
                timeout_ms,
                operation_budget: budget,
                mr_rounds,
                enable_ecm: ecm,
                ecm_b1,
                ecm_max_curves: ecm_curves,
                ..Default::default()
            };
            match factor_to_json(&input, Some(config)) {
                Ok(out) => {
                    if json {
                        println!("{out}");
                    } else {
                        println!("{out}");
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            }
        }
        Commands::Primality {
            input,
            rounds,
            seed,
            json: _,
        } => match primality_to_json(&input, rounds, seed) {
            Ok(out) => println!("{out}"),
            Err(e) => {
                eprintln!("error: {e}");
                process::exit(1);
            }
        },
        Commands::Verify { file } => {
            let data = if let Some(path) = file {
                std::fs::read_to_string(path).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                })
            } else {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf).unwrap_or_else(|e| {
                    eprintln!("error: {e}");
                    process::exit(1);
                });
                buf
            };
            match verify_json(&data) {
                Ok(v) => {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                }
                Err(e) => {
                    eprintln!("verification failed: {e}");
                    process::exit(1);
                }
            }
        }
        Commands::Info => {
            println!("BabelPrime {VERSION}");
            println!("schema_version: 0.1.0");
            println!(
                "algorithms: trial_division, perfect_power, miller_rabin, pollard_rho, pollard_p1, ecm_stage1"
            );
            println!(
                "future (not operational): ecm_stage2, quadratic_sieve, gnfs, quantum_order_finding"
            );
            println!("checkpoint/resume: available (curve-boundary; SHA-256 integrity)");
        }
    }
}
