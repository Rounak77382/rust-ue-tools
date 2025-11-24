//! RETOC CLI - UTOC file manipulation tool
//!
//! This executable provides the same functionality as the original retoc_cli tool
//! but using pure Rust implementation from the rust-ue-tools library.

use std::path::PathBuf;
use clap::{Parser, Subcommand, Args};
use std::process;

use rust_ue_tools::{Unpacker, UtocListOptions, error::Result};

/// RETOC CLI - Unreal Engine UTOC file manipulation
#[derive(Parser, Debug)]
#[command(name = "retoc")]
#[command(about = "UTOC file manipulation tool - Pure Rust implementation")]
#[command(version = "1.0.0")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List contents of UTOC files (equivalent to original retoc_cli list)
    List {
        /// Path to the .utoc file to list
        #[arg(value_name = "UTOC_FILE")]
        utoc_file: PathBuf,
        
        /// Output in JSON format
        #[arg(long = "json")]
        json: bool,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
        
        /// Quiet mode (minimal output)
        #[arg(short = 'q', long = "quiet")]
        quiet: bool,
    },
    
    /// Show UTOC file information (equivalent to original retoc_cli info)
    Info {
        /// Path to the .utoc file to analyze
        #[arg(value_name = "UTOC_FILE")]
        utoc_file: PathBuf,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
        
        /// Output in JSON format
        #[arg(long = "json")]
        json: bool,
    },
    
    /// Extract files from UTOC archives
    Extract {
        /// Path to the .utoc file to extract
        #[arg(value_name = "UTOC_FILE")]
        utoc_file: PathBuf,
        
        /// Output directory for extracted files
        #[arg(short = 'o', long = "output")]
        output: PathBuf,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
        
        /// Force overwrite of existing files
        #[arg(short = 'f', long = "force")]
        force: bool,
        
        /// Quiet mode (minimal output)
        #[arg(short = 'q', long = "quiet")]
        quiet: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::List { utoc_file, json, key, quiet } => {
            handle_list(utoc_file, json, key, quiet);
        }
        Commands::Info { utoc_file, key, json } => {
            handle_info(utoc_file, key, json);
        }
        Commands::Extract { utoc_file, output, key, force, quiet } => {
            handle_extract(utoc_file, output, key, force, quiet);
        }
    }
}

fn handle_list(utoc_file: PathBuf, json: bool, key: Option<String>, quiet: bool) {
    // Validate input file
    if !utoc_file.exists() {
        eprintln!("Error: UTOC file not found: {}", utoc_file.display());
        process::exit(1);
    }
    
    if !utoc_file.extension().map_or(false, |ext| ext == "utoc") {
        eprintln!("Error: File must have .utoc extension");
        process::exit(1);
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Build options
    let mut options = UtocListOptions::new()
        .with_json_format(json);
    
    if let Some(ref k) = key {
        options = options.with_aes_key(k);
    }
    
    // Perform listing
    if !quiet && !json {
        println!("Listing contents of {}", utoc_file.display());
    }
    
    match unpacker.list_utoc(&utoc_file, &options) {
        Ok(asset_paths) => {
            if json {
                // JSON output format
                let output = serde_json::json!({
                    "file": utoc_file.to_string_lossy(),
                    "asset_count": asset_paths.len(),
                    "assets": asset_paths.iter().map(|p| p.as_str()).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
                    eprintln!("Error serializing JSON: {}", e);
                    process::exit(1);
                }));
            } else if !quiet {
                // Simple text output
                println!("Found {} assets:", asset_paths.len());
                for asset in &asset_paths {
                    println!("  {}", asset.as_str());
                }
            }
        }
        Err(e) => {
            if json {
                let error_output = serde_json::json!({
                    "error": e.to_string(),
                    "file": utoc_file.to_string_lossy()
                });
                println!("{}", serde_json::to_string_pretty(&error_output).unwrap_or_else(|_| {
                    eprintln!("Error serializing JSON: {}", e);
                    process::exit(1);
                }));
            } else {
                eprintln!("Error listing UTOC file: {}", e);
            }
            process::exit(1);
        }
    }
}

fn handle_info(utoc_file: PathBuf, key: Option<String>, json: bool) {
    // Validate input file
    if !utoc_file.exists() {
        eprintln!("Error: UTOC file not found: {}", utoc_file.display());
        process::exit(1);
    }
    
    if !utoc_file.extension().map_or(false, |ext| ext == "utoc") {
        eprintln!("Error: File must have .utoc extension");
        process::exit(1);
    }
    
    println!("Analyzing UTOC file: {}", utoc_file.display());
    
    // Use the list function and format as info
    let mut unpacker = Unpacker::new();
    let mut options = UtocListOptions::new()
        .with_json_format(true);
    
    if let Some(ref k) = key {
        options = options.with_aes_key(k);
    }
    
    match unpacker.list_utoc(&utoc_file, &options) {
        Ok(asset_paths) => {
            let info_output = serde_json::json!({
                "file": utoc_file.to_string_lossy(),
                "file_size": utoc_file.metadata().map(|m| m.len()).unwrap_or(0),
                "modified": utoc_file.metadata()
                    .and_then(|m| m.modified())
                    .map_or_else(|_| "unknown".to_string(), |t| format!("{:?}", t)),
                "asset_count": asset_paths.len(),
                "compression_methods": [],
                "encryption": key.is_some(),
                "assets": asset_paths.iter().map(|p| p.as_str()).collect::<Vec<_>>()
            });
            
            println!("{}", serde_json::to_string_pretty(&info_output).unwrap_or_else(|e| {
                eprintln!("Error serializing JSON: {}", e);
                process::exit(1);
            }));
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "error": e.to_string(),
                "file": utoc_file.to_string_lossy()
            });
            println!("{}", serde_json::to_string_pretty(&error_output).unwrap_or_else(|_| {
                eprintln!("Error serializing JSON: {}", e);
                process::exit(1);
            }));
            process::exit(1);
        }
    }
}

fn handle_extract(utoc_file: PathBuf, output: PathBuf, key: Option<String>, force: bool, quiet: bool) {
    eprintln!("Error: UTOC extraction not yet implemented");
    process::exit(1);
}