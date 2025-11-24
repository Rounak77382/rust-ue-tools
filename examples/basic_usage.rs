//! Basic usage example for the unified UE tools library
//!
//! This example demonstrates how to use the library to:
//! 1. Extract asset paths from an archive file containing pak/utoc files
//! 2. Unpack a single pak file
//! 3. List contents of a.utoc file
//!
//! USAGE:
//!   basic_usage <archive_path> <pak_path> <utoc_path> [aes_key]
//!
//! EXAMPLES:
//!   basic_usage mod.zip mod.pak mod.utoc
//!   basic_usage mod.zip mod.pak mod.utoc 0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74

use rust_ue_tools::{Unpacker, PakUnpackOptions, UtocListOptions};
use std::path::Path;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    println!("UE Tools Rust Library - Basic Usage Example");
    println!("==========================================");
    
    if args.len() < 4 {
        println!("Usage: {} <archive_path> <pak_path> <utoc_path> [aes_key]", args[0]);
        println!("Example: {} mod.zip mod.pak mod.utoc", args[0]);
        println!();
        println!("This example will:");
        println!("1. Extract asset paths from archive (if provided)");
        println!("2. Unpack pak file (if provided)");
        println!("3. List utoc contents (if provided)");
        return Ok(());
    }

    let archive_path = &args[1];
    let pak_path = &args[2];
    let utoc_path = &args[3];
    let aes_key = args.get(4).map(|s| s.as_str());

    // Create the unpacker instance
    let mut unpacker = Unpacker::new();

    // Example 1: Extract files from an archive (ZIP or RAR)
    if Path::new(archive_path).exists() {
        println!("\n1. Extracting files from archive: {}", archive_path);
        match unpacker.extract_asset_paths_from_archive(archive_path, aes_key, false) {
            Ok(asset_paths) => {
                println!("Found {} asset paths:", asset_paths.len());
                for asset in &asset_paths {
                    println!("  - {}", asset);
                }
            }
            Err(e) => {
                println!("Error extracting from archive: {}", e);
            }
        }
    } else {
        println!("\n1. Archive file not found: {}", archive_path);
    }

    // Example 2: Unpack a single pak file
    if Path::new(pak_path).exists() {
        println!("\n2. Unpacking pak file: {}", pak_path);
        let output_dir = format!("unpacked_{}", Path::new(pak_path).file_stem().unwrap().to_string_lossy());
        
        let options = PakUnpackOptions::new()
            .with_aes_key(aes_key.unwrap_or_default())
            .with_strip_prefix("../../../")
            .with_force(true)
            .with_quiet(false);
        
        match unpacker.unpack_pak(pak_path, &output_dir, &options) {
            Ok(asset_paths) => {
                println!("Unpacked {} files to {}", asset_paths.len(), output_dir);
                println!("First few assets:");
                for asset in asset_paths.iter().take(5) {
                    println!("  - {}", asset);
                }
                if asset_paths.len() > 5 {
                    println!("  ... and {} more", asset_paths.len() - 5);
                }
            }
            Err(e) => {
                println!("Error unpacking pak: {}", e);
            }
        }
    } else {
        println!("\n2. Pak file not found: {}", pak_path);
    }

    // Example 3: List contents of a.utoc file
    if Path::new(utoc_path).exists() {
        println!("\n3. Listing.utoc file contents: {}", utoc_path);
        
        let options = UtocListOptions::new()
            .with_aes_key(aes_key.unwrap_or_default())
            .with_json_format(false);
        
        match unpacker.list_utoc(utoc_path, &options) {
            Ok(asset_paths) => {
                println!("Found {} assets in.utoc:", asset_paths.len());
                for asset in &asset_paths {
                    println!("  - {}", asset);
                }
            }
            Err(e) => {
                println!("Error listing.utoc: {}", e);
            }
        }
    } else {
        println!("\n3. UTOC file not found: {}", utoc_path);
    }

    println!("\nExample completed!");
    Ok(())
}