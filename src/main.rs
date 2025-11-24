//! Main CLI entry point for UE tools
//!
//! This executable provides a unified command-line interface for all Unreal Engine
//! file manipulation tools, combining the functionality of repak and retoc_cli.

use rust_ue_tools::cli;

fn main() {
    if let Err(e) = cli::run_cli() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}