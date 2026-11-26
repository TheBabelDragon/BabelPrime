//! BabelPrime CLI.

use babelprime_core::{factorize_str, primality_to_json, verify_result, FactorJobConfig, VERSION};
use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "babelprime",
    version = VERSION,
    about = "Verifiable integer factorization and primality engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Factor {
        input: String,
        #[arg(long, default_value_t = 0xB4B31)]
        seed: u64,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
        #[arg(long, default_value_t = 10_000_000)]
        budget: u64,
        #[arg(long, default_value_t = 16)]
        mr_rounds: u32,
        #[arg(long, default_value_t = true)]
        json: bool,
    },
    Primality {
        input: String,
        #[arg(long, default_value_t = 16)]
        rounds: u32,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    Verify {
        path: Option<String>,
    },
    Info,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Factor {
            input,
            seed,
            timeout_ms,
            budget,
            mr_rounds,
            json,
        } => {
            let config = FactorJobConfig {
                seed,
                timeout_ms,
                operation_budget: budget,
                mr_rounds,
                ..Default::default()
            };
            match factorize_str(&input, Some(config)) {
                Ok(res) => {
                    if json {
                        match serde_json::to_string_pretty(&res) {
                            Ok(s) => {
                                println!("{s}");
                                if let Err(e) = verify_result(&res) {
                                    eprintln!("warning: verification: {e}");
                                    return ExitCode::from(2);
                                }
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                eprintln!("json error: {e}");
                                ExitCode::FAILURE
                            }
                        }
                    } else {
                        println!("{res:?}");
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Primality {
            input,
            rounds,
            seed,
        } => match primality_to_json(&input, rounds, seed) {
            Ok(s) => {
                println!("{s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Commands::Verify { path } => {
            let text = match path {
                Some(p) => std::fs::read_to_string(p).unwrap_or_else(|e| {
                    eprintln!("read error: {e}");
                    std::process::exit(1);
                }),
                None => {
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf).ok();
                    buf
                }
            };
            match babelprime_core::verify_json(&text) {
                Ok(()) => {
                    println!("OK");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("FAIL: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Commands::Info => {
            println!("BabelPrime {VERSION}");
            println!("schema_version: {}", babelprime_core::SCHEMA_VERSION);
            println!(
                "algorithms: trial_division, perfect_power, miller_rabin, pollard_rho, pollard_p1"
            );
            println!("future (not operational): ecm, quadratic_sieve, gnfs, quantum_order_finding");
            ExitCode::SUCCESS
        }
    }
}
