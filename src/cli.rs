//! Command-line interface for UE file manipulation tools
//!
//! This module provides command-line interfaces that replicate the functionality
//! of repak and retoc_cli tools, but using pure Rust implementation.

use std::path::PathBuf;
use clap::{Parser, Subcommand, Args};
use serde_json;

use crate::error::{Result, UeToolError};
use crate::{Unpacker, PakUnpackOptions, UtocListOptions, AssetPath};

/// CLI arguments for UE file manipulation tools
#[derive(Parser, Debug)]
#[command(name = "ue-tools")]
#[command(about = "Unreal Engine file manipulation tools - Pure Rust implementation")]
#[command(version = "1.0.0")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Unpack PAK files
    Unpack(UnpackArgs),
    /// List contents of UTOC files  
    Retoc(RetocArgs),
    /// Extract asset paths from archives
    Extract(ExtractArgs),
}

#[derive(Args, Debug)]
struct UnpackArgs {
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
    #[arg(long = "strip-prefix")]
    strip_prefix: Option<String>,
}

#[derive(Args, Debug)]
struct RetocArgs {
    #[command(subcommand)]
    action: RetocAction,
}

#[derive(Subcommand, Debug)]
enum RetocAction {
    /// List contents of a UTOC file
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
    },
    
    /// Extract detailed information about UTOC file
    Info {
        /// Path to the .utoc file to analyze
        #[arg(value_name = "UTOC_FILE")]
        utoc_file: PathBuf,
        
        /// AES encryption key (hex format)
        #[arg(short = 'k', long = "key")]
        key: Option<String>,
    },
}

#[derive(Args, Debug)]
struct ExtractArgs {
    /// Path to the archive file (ZIP or RAR)
    #[arg(value_name = "ARCHIVE_FILE")]
    archive_file: PathBuf,
    
    /// AES encryption key (hex format)
    #[arg(short = 'k', long = "key")]
    key: Option<String>,
    
    /// Keep temporary files after extraction
    #[arg(long = "keep-temp")]
    keep_temp: bool,
    
    /// Quiet mode (minimal output)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
}

/// Main CLI entry point
pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Unpack(args) => {
            handle_unpack(args)
        }
        Commands::Retoc(args) => {
            handle_retoc(args)
        }
        Commands::Extract(args) => {
            handle_extract(args)
        }
    }
}

/// Handle unpacking of PAK files (replicates: unpack <pak_file> -o <output_dir> -q -f)
fn handle_unpack(args: UnpackArgs) -> Result<()> {
    // Validate input file
    if !args.pak_file.exists() {
        return Err(UeToolError::file_not_found(&args.pak_file));
    }
    
    if !args.pak_file.extension().map_or(false, |ext| ext == "pak") {
        return Err(UeToolError::invalid_format("File must have .pak extension"));
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Build options
    let mut options = PakUnpackOptions::new()
        .with_force(args.force)
        .with_quiet(args.quiet);
    
    if let Some(ref key) = args.key {
        options = options.with_aes_key(key);
    }
    
    if let Some(ref prefix) = args.strip_prefix {
        options = options.with_strip_prefix(prefix);
    }
    
    // Perform unpacking
    if !args.quiet {
        println!("Unpacking {} to {}", args.pak_file.display(), args.output.display());
    }
    
    match unpacker.unpack_pak(&args.pak_file, &args.output, &options) {
        Ok(asset_paths) => {
            if !args.quiet {
                println!("Successfully unpacked {} files", asset_paths.len());
                
                // Show summary of extracted files
                for asset in asset_paths.iter().take(10) {
                    println!("  {}", asset.as_str());
                }
                
                if asset_paths.len() > 10 {
                    println!("  ... and {} more files", asset_paths.len() - 10);
                }
            }
            Ok(())
        }
        Err(e) => {
            if !args.quiet {
                eprintln!("Error unpacking PAK file: {}", e);
            }
            Err(e)
        }
    }
}

/// Handle UTOC file operations (replicates: retoc_cli list <utoc_file> --json)
fn handle_retoc(args: RetocArgs) -> Result<()> {
    match args.action {
        RetocAction::List { utoc_file, json, key } => {
            handle_retoc_list(utoc_file, json, key)
        }
        RetocAction::Info { utoc_file, key } => {
            handle_retoc_info(utoc_file, key)
        }
    }
}

fn handle_retoc_list(utoc_file: PathBuf, json: bool, key: Option<String>) -> Result<()> {
    // Validate input file
    if !utoc_file.exists() {
        return Err(UeToolError::file_not_found(&utoc_file));
    }
    
    if !utoc_file.extension().map_or(false, |ext| ext == "utoc") {
        return Err(UeToolError::invalid_format("File must have .utoc extension"));
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
    if !json {
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
                println!("{}", serde_json::to_string_pretty(&output).map_err(|e| UeToolError::SerializationError(format!("Failed to serialize JSON: {}", e)))?);
            } else {
                // Simple text output
                println!("Found {} assets:", asset_paths.len());
                for asset in &asset_paths {
                    println!("  {}", asset.as_str());
                }
            }
            Ok(())
        }
        Err(e) => {
            if json {
                let error_output = serde_json::json!({
                    "error": e.to_string(),
                    "file": utoc_file.to_string_lossy()
                });
                println!("{}", serde_json::to_string_pretty(&error_output).map_err(|e| UeToolError::SerializationError(format!("Failed to serialize JSON: {}", e)))?);
            } else {
                eprintln!("Error listing UTOC file: {}", e);
            }
            Err(e)
        }
    }
}

fn handle_retoc_info(utoc_file: PathBuf, key: Option<String>) -> Result<()> {
    // Validate input file
    if !utoc_file.exists() {
        return Err(UeToolError::file_not_found(&utoc_file));
    }
    
    if !utoc_file.extension().map_or(false, |ext| ext == "utoc") {
        return Err(UeToolError::invalid_format("File must have .utoc extension"));
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Build options
    let mut options = UtocListOptions::new()
        .with_json_format(true);
    
    if let Some(ref k) = key {
        options = options.with_aes_key(k);
    }
    
    println!("Analyzing UTOC file: {}", utoc_file.display());
    
    // For now, use the standard list function and format as info
    match unpacker.list_utoc(&utoc_file, &options) {
        Ok(asset_paths) => {
            let info_output = serde_json::json!({
                "file": utoc_file.to_string_lossy(),
                "file_size": utoc_file.metadata().map(|m| m.len()).unwrap_or(0),
                "modified": utoc_file.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok().map(|t| format!("{:?}", t)))
                    .unwrap_or_else(|| "unknown".to_string()),
                "asset_count": asset_paths.len(),
                "compression_methods": [],
                "encryption": key.is_some(),
                "assets": asset_paths.iter().map(|p| p.as_str()).collect::<Vec<_>>()
            });
            
            println!("{}", serde_json::to_string_pretty(&info_output)?);
            Ok(())
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "error": e.to_string(),
                "file": utoc_file.to_string_lossy()
            });
            println!("{}", serde_json::to_string_pretty(&error_output)?);
            Err(e)
        }
    }
}

/// Handle extraction from archive files
fn handle_extract(args: ExtractArgs) -> Result<()> {
    // Validate input file
    if !args.archive_file.exists() {
        return Err(UeToolError::file_not_found(&args.archive_file));
    }
    
    // Check if it's a supported archive type
    let ext = args.archive_file.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    if !matches!(ext.as_str(), "zip" | "rar") {
        return Err(UeToolError::invalid_argument(format!(
            "Unsupported archive type: {}. Only ZIP and RAR are supported.", ext
        )));
    }
    
    // Create unpacker instance
    let mut unpacker = Unpacker::new();
    
    // Perform extraction
    if !args.quiet {
        println!("Extracting asset paths from {}", args.archive_file.display());
    }
    
    match unpacker.extract_asset_paths_from_archive(&args.archive_file, args.key.as_deref(), args.keep_temp) {
        Ok(asset_paths) => {
            if !args.quiet {
                println!("Found {} asset paths:", asset_paths.len());
                for asset in asset_paths.iter().take(20) {
                    println!("  {}", asset.as_str());
                }
                if asset_paths.len() > 20 {
                    println!("  ... and {} more files", asset_paths.len() - 20);
                }
            }
            
            // Output list for scripting
            for asset in &asset_paths {
                println!("{}", asset.as_str());
            }
            
            Ok(())
        }
        Err(e) => {
            if !args.quiet {
                eprintln!("Error extracting from archive: {}", e);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cli_parsing() {
        // Test basic CLI parsing
        let cli = Cli::parse_from(&["ue-tools", "unpack", "test.pak", "-o", "output", "-q", "-f"]);
        
        match cli.command {
            Commands::Unpack(args) => {
                assert_eq!(args.pak_file, PathBuf::from("test.pak"));
                assert_eq!(args.output, PathBuf::from("output"));
                assert!(args.quiet);
                assert!(args.force);
            }
            _ => panic!("Expected Unpack command"),
        }
    }

    #[test]
    fn test_retoc_cli_parsing() {
        let cli = Cli::parse_from(&["ue-tools", "retoc", "list", "test.utoc", "--json"]);
        
        match cli.command {
            Commands::Retoc(args) => {
                match args.action {
                    RetocAction::List { utoc_file, json, .. } => {
                        assert_eq!(utoc_file, PathBuf::from("test.utoc"));
                        assert!(json);
                    }
                    _ => panic!("Expected List subcommand"),
                }
            }
            _ => panic!("Expected Retoc command"),
        }
    }
}