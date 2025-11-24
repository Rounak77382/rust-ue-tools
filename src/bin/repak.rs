//! REPAK CLI - PAK file manipulation tool
//!
//! This executable provides the same functionality as the original repak tool
//! but using pure Rust implementation from the rust-ue-tools library.

use std::path::PathBuf;
use clap::{Parser, Subcommand, Args};
use std::process;

use rust_ue_tools::{Unpacker, PakUnpackOptions, error::Result};

/// REPAK CLI - Unreal Engine PAK file manipulation
#[derive(Parser, Debug)]
#[command(name = "repak")]
#[command(about = "PAK file manipulation tool - Pure Rust implementation")]
#[command(version = "1.0.0")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Unpack PAK files (equivalent to original repak unpack)
    Unpack {
        /// Path to the .pak file to unpack
        #[arg(value_name = "PAK_FILE")]
        pak_file: PathBuf,
        
        /// Output directory for extracted files
        #[arg(short = 'o', long = "output")]
        output: PathBuf,
        
        /// Quiet mode (minimal output)
        #[arg(short = 'q', long = "quiet")]
        quiet: bool,
        
        /// Force overwrite of existing files
        #[arg(short = 'f', long = "force")]
        force: bool,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
        
        /// Strip path prefix from extracted files
        #[arg(long = "strip-prefix", default_value = "../../../")]
        strip_prefix: String,
    },
    
    /// List files in a PAK (equivalent to original repak list)
    List {
        /// Path to the .pak file to list
        #[arg(value_name = "PAK_FILE")]
        pak_file: PathBuf,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
        
        /// Output in JSON format
        #[arg(long = "json")]
        json: bool,
        
        /// Filter by pattern
        #[arg(short = 'p', long = "pattern")]
        pattern: Option<String>,
    },
    
    /// Show PAK file information
    Info {
        /// Path to the .pak file to analyze
        #[arg(value_name = "PAK_FILE")]
        pak_file: PathBuf,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Unpack { pak_file, output, quiet, force, key, strip_prefix } => {
            handle_unpack(pak_file, output, quiet, force, key, strip_prefix);
        }
        Commands::List { pak_file, key, json, pattern } => {
            handle_list(pak_file, key, json, pattern);
        }
        Commands::Info { pak_file, key } => {
            handle_info(pak_file, key);
        }
    }
}

fn handle_unpack(pak_file: PathBuf, output: PathBuf, quiet: bool, force: bool, key: Option<String>, strip_prefix: String) {
    // Validate input file
    if !pak_file.exists() {
        eprintln!("Error: PAK file not found: {}", pak_file.display());
        process::exit(1);
    }
    
    if !pak_file.extension().map_or(false, |ext| ext == "pak") {
        eprintln!("Error: File must have .pak extension");
        process::exit(1);
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Build options
    let mut options = PakUnpackOptions::new()
        .with_force(force)
        .with_quiet(quiet)
        .with_strip_prefix(strip_prefix);
    
    if let Some(ref k) = key {
        options = options.with_aes_key(k);
    }
    
    // Perform unpacking
    if !quiet {
        println!("Unpacking {} to {}", pak_file.display(), output.display());
    }
    
    match unpacker.unpack_pak(&pak_file, &output, &options) {
        Ok(asset_paths) => {
            if !quiet {
                println!("Successfully unpacked {} files", asset_paths.len());
                
                // Show summary of extracted files
                for asset in asset_paths.iter().take(10) {
                    println!("  {}", asset.as_str());
                }
                
                if asset_paths.len() > 10 {
                    println!("  ... and {} more files", asset_paths.len() - 10);
                }
            }
        }
        Err(e) => {
            if !quiet {
                eprintln!("Error unpacking PAK file: {}", e);
            }
            process::exit(1);
        }
    }
}

fn handle_list(pak_file: PathBuf, key: Option<String>, json: bool, pattern: Option<String>) {
    // Validate input file
    if !pak_file.exists() {
        eprintln!("Error: PAK file not found: {}", pak_file.display());
        process::exit(1);
    }
    
    if !pak_file.extension().map_or(false, |ext| ext == "pak") {
        eprintln!("Error: File must have .pak extension");
        process::exit(1);
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Build options
    let mut options = PakUnpackOptions::new()
        .with_quiet(true);
    
    if let Some(ref k) = key {
        options = options.with_aes_key(k);
    }
    
    // Apply pattern filter if provided
    if let Some(ref pat) = pattern {
        match glob::Pattern::new(pat) {
            Ok(pattern) => {
                options = options.with_include_patterns(vec![pattern]);
            }
            Err(e) => {
                eprintln!("Error: Invalid pattern '{}': {}", pat, e);
                process::exit(1);
            }
        }
    }
    
    // List files
    if !json {
        println!("Listing contents of {}", pak_file.display());
    }
    
    match unpacker.pak_unpacker.list_files(&pak_file, &options) {
        Ok(file_paths) => {
            if json {
                // JSON output format
                let output = serde_json::json!({
                    "file": pak_file.to_string_lossy(),
                    "file_count": file_paths.len(),
                    "files": file_paths.iter().map(|p| p.as_str()).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
                    eprintln!("Error serializing JSON: {}", e);
                    process::exit(1);
                }));
            } else {
                // Simple text output
                println!("Found {} files:", file_paths.len());
                for file in file_paths {
                    println!("  {}", file.as_str());
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing PAK file: {}", e);
            process::exit(1);
        }
    }
}

fn handle_info(pak_file: PathBuf, key: Option<String>) {
    // Validate input file
    if !pak_file.exists() {
        eprintln!("Error: PAK file not found: {}", pak_file.display());
        process::exit(1);
    }
    
    if !pak_file.extension().map_or(false, |ext| ext == "pak") {
        eprintln!("Error: File must have .pak extension");
        process::exit(1);
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Build options
    let mut options = PakUnpackOptions::new()
        .with_quiet(true);
    
    if let Some(ref k) = key {
        options = options.with_aes_key(k);
    }
    
    println!("Getting info for {}", pak_file.display());
    
    match unpacker.pak_unpacker.get_info(&pak_file, &options) {
        Ok(info) => {
            println!("PAK File Information:");
            println!("  File: {}", info["file_path"]);
            println!("  Files: {}", info["file_count"]);
            if let Some(file_names) = info["file_names"].as_array() {
                println!("  File Names (first 10):");
                for file in file_names.iter().take(10) {
                    println!("    {}", file);
                }
                if file_names.len() > 10 {
                    println!("    ... and {} more files", file_names.len() - 10);
                }
            }
        }
        Err(e) => {
            eprintln!("Error getting PAK file info: {}", e);
            process::exit(1);
        }
    }
}