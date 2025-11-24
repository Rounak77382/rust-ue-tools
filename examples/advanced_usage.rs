//! Advanced example demonstrating how to replace the Python zip_to_asset_paths.py functionality
//!
//! This example shows how to:
//! 1. Extract asset paths from archive files (replaces `extract_uasset_paths_from_zip`)
//! 2. Extract pak asset maps from folders (replaces `extract_pak_asset_map_from_folder`)
//! 3. Handle both pak and.utoc files with proper error handling
//! 4. Use progress reporting for long operations
//!
//! USAGE:
//!   advanced_usage <command> <path> [aes_key]
//!
//! COMMANDS:
//!   archive <path>           - Extract asset paths from archive file
//!   folder <path>            - Process all UE files in folder
//!   batch <file1> [file2...] - Process multiple files
//!
//! EXAMPLES:
//!   advanced_usage archive mod.zip
//!   advanced_usage folder ./game_files/
//!   advanced_usage batch file1.pak file2.utoc file3.zip

use rust_ue_tools::{Unpacker, PakUnpackOptions, UtocListOptions, AssetPath, ProgressInfo};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    println!("UE Tools Rust Library - Advanced Python Replacement Example");
    println!("==========================================================");
    
    if args.len() < 3 {
        println!("Usage: {} <command> <path> [aes_key]", args[0]);
        println!();
        println!("Commands:");
        println!("  archive <path>    - Extract asset paths from archive file");
        println!("  folder <path>     - Process all UE files in folder");
        println!("  batch <file1> [file2...] - Process multiple files");
        println!();
        println!("Examples:");
        println!("  {} archive mod.zip", args[0]);
        println!("  {} folder ./game_files/", args[0]);
        println!("  {} batch file1.pak file2.utoc", args[0]);
        return Ok(());
    }

    let command = &args[1];
    let aes_key = args.get(3).map(|s| s.as_str());

    match command.as_str() {
        "archive" => {
            if args.len() < 3 {
                println!("Error: archive command requires a path");
                return Ok(());
            }
            let archive_path = &args[2];
            run_archive_example(archive_path, aes_key)?;
        }
        "folder" => {
            if args.len() < 3 {
                println!("Error: folder command requires a path");
                return Ok(());
            }
            let folder_path = &args[2];
            run_folder_example(folder_path, aes_key)?;
        }
        "batch" => {
            if args.len() < 3 {
                println!("Error: batch command requires at least one file path");
                return Ok(());
            }
            let file_paths: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();
            run_batch_example(&file_paths, aes_key)?;
        }
        _ => {
            println!("Error: Unknown command '{}'. Use 'archive', 'folder', or 'batch'", command);
        }
    }

    println!("\nAdvanced example completed!");
    Ok(())
}

/// Run the archive extraction example
fn run_archive_example(archive_path: &str, aes_key: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n1. Extracting asset paths from archive (replaces Python function)...");
    
    let unpacker = Unpacker::new();
    match extract_uasset_paths_from_archive(&unpacker, archive_path, aes_key, false) {
        Ok(asset_paths) => {
            println!("Found {} asset paths:", asset_paths.len());
            for asset in &asset_paths {
                println!("  - {}", asset);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

/// Run the folder processing example
fn run_folder_example(folder_path: &str, aes_key: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n2. Extracting pak asset map from folder (replaces Python function)...");
    
    let unpacker = Unpacker::new();
    match extract_pak_asset_map_from_folder(&unpacker, folder_path, aes_key) {
        Ok(asset_map) => {
            println!("Asset map for {} pak files:", asset_map.len());
            for (pak_name, assets) in &asset_map {
                println!("  {}: {} assets", pak_name, assets.len());
                for asset in assets.iter().take(5) { // Show first 5 assets
                    println!("    - {}", asset);
                }
                if assets.len() > 5 {
                    println!("    ... and {} more", assets.len() - 5);
                }
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

/// Run the batch processing example
fn run_batch_example(file_paths: &[&str], aes_key: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n3. Processing multiple files in batch...");
    
    let unpacker = Unpacker::new();
    match process_multiple_files(&unpacker, file_paths, aes_key) {
        Ok(results) => {
            println!("Successfully processed {} files:", results.len());
            for (file_name, assets) in &results {
                println!("  {}: {} assets", file_name, assets.len());
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    Ok(())
}

/// Rust equivalent of Python's extract_uasset_paths_from_zip function
/// Updated to use the correct function name: extract_asset_paths_from_archive
pub fn extract_uasset_paths_from_archive(
    unpacker: &Unpacker,
    archive_path: &str,
    aes_key: Option<&str>,
    keep_temp: bool,
) -> Result<Vec<AssetPath>, Box<dyn std::error::Error>> {
    println!("Extracting asset paths from archive: {}", archive_path);

    let path = Path::new(archive_path);
    if !path.exists() {
        return Err(format!("Archive file not found: {}", archive_path).into());
    }

    // Use the unified library's archive extraction
    let asset_paths = unpacker.extract_asset_paths_from_archive(path, aes_key, keep_temp)
        .map_err(|e| format!("Failed to extract asset paths: {}", e))?;

    Ok(asset_paths)
}

/// Rust equivalent of Python's extract_pak_asset_map_from_folder function
pub fn extract_pak_asset_map_from_folder(
    unpacker: &Unpacker,
    folder_path: &str,
    aes_key: Option<&str>,
) -> Result<HashMap<String, Vec<AssetPath>>, Box<dyn std::error::Error>> {
    println!("Extracting pak asset map from folder: {}", folder_path);

    let base = Path::new(folder_path);
    if !base.exists() || !base.is_dir() {
        return Err(format!("Folder not found: {}", folder_path).into());
    }

    let mut result: HashMap<String, Vec<AssetPath>> = HashMap::new();

    // Process classic .pak files
    println!("Processing classic .pak files...");
    let pak_files: Vec<PathBuf> = walkdir::WalkDir::new(base)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "pak"))
        .map(|entry| entry.path().to_path_buf())
        .collect();

    for pak in &pak_files {
        let output_dir = pak.with_suffix("");
        let options = PakUnpackOptions::new()
            .with_aes_key(aes_key.unwrap_or_default())
            .with_strip_prefix("../../../")
            .with_force(true)
            .with_quiet(true);

        match unpacker.unpack_pak(pak, &output_dir, &options) {
            Ok(assets) => {
                result.insert(pak.file_stem().unwrap().to_string_lossy().to_string(), assets);
            }
            Err(e) => {
                println!("Warning: Failed to unpack {}: {}", pak.display(), e);
            }
        }
    }

    // Process IoStore (.utoc + .ucas) files
    println!("Processing IoStore (.utoc) files...");
    let utoc_files: Vec<PathBuf> = walkdir::WalkDir::new(base)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "utoc"))
        .map(|entry| entry.path().to_path_buf())
        .collect();

    for utoc in &utoc_files {
        let pak_name = utoc.file_stem().unwrap().to_string_lossy().to_string();
        let options = UtocListOptions::new()
            .with_aes_key(aes_key.unwrap_or_default())
            .with_json_format(false);

        match unpacker.list_utoc(utoc, &options) {
            Ok(assets) => {
                result.insert(pak_name, assets);
            }
            Err(e) => {
                println!("Warning: Failed to list {}: {}", utoc.display(), e);
            }
        }
    }

    Ok(result)
}

/// Advanced example showing batch processing with progress tracking
pub fn process_multiple_files(
    unpacker: &Unpacker,
    file_paths: &[&str],
    aes_key: Option<&str>,
) -> Result<HashMap<String, Vec<AssetPath>>, Box<dyn std::error::Error>> {
    println!("Processing {} files with progress tracking...", file_paths.len());

    let progress = Arc::new(Mutex::new(0));
    let total = file_paths.len();
    let results: Arc<Mutex<HashMap<String, Vec<AssetPath>>>> = Arc::new(Mutex::new(HashMap::new()));

    // Process files in parallel
    std::thread::scope(|s| {
        let handles: Vec<_> = file_paths.iter().map(|file_path| {
            let unpacker = unpacker;
            let aes_key = aes_key;
            let progress = Arc::clone(&progress);
            let results = Arc::clone(&results);
            
            s.spawn(move || {
                let path = Path::new(file_path);
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                
                match ext {
                    "pak" => {
                        let output_dir = path.with_suffix("");
                        let options = PakUnpackOptions::new()
                            .with_aes_key(aes_key.unwrap_or_default())
                            .with_strip_prefix("../../../")
                            .with_force(true)
                            .with_quiet(true);

                        match unpacker.unpack_pak(path, &output_dir, &options) {
                            Ok(assets) => {
                                let mut results = results.lock().unwrap();
                                results.insert(path.file_stem().unwrap().to_string_lossy().to_string(), assets);
                            }
                            Err(e) => {
                                println!("Warning: Failed to process {}: {}", file_path, e);
                            }
                        }
                    }
                    "utoc" => {
                        let options = UtocListOptions::new()
                            .with_aes_key(aes_key.unwrap_or_default())
                            .with_json_format(false);

                        match unpacker.list_utoc(path, &options) {
                            Ok(assets) => {
                                let mut results = results.lock().unwrap();
                                results.insert(path.file_stem().unwrap().to_string_lossy().to_string(), assets);
                            }
                            Err(e) => {
                                println!("Warning: Failed to process {}: {}", file_path, e);
                            }
                        }
                    }
                    _ => {
                        println!("Warning: Unsupported file type for {}", file_path);
                    }
                }

                // Update progress
                let mut progress = progress.lock().unwrap();
                *progress += 1;
                println!("Progress: {}/{} ({:.1}%)", *progress, total, (*progress as f64 / total as f64) * 100.0);
            })
        }).collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
    });

    Ok(Arc::try_unwrap(results).unwrap().into_inner().unwrap())
}

/// Example of how to handle the extensions that the original Python code looked for
pub fn filter_asset_extensions(asset_paths: &[AssetPath]) -> Vec<&AssetPath> {
    const EXTENSIONS_TO_PRINT: &[&str] = &[
        "uasset", "umap", "bnk", "json", "wem", "fbx", "obj", "glb", "gltf", 
        "ini", "wav", "mp3", "ogg", "uplugin", "usf"
    ];

    asset_paths
        .iter()
        .filter(|asset| {
            asset.extension()
                .map(|ext| EXTENSIONS_TO_PRINT.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect()
}

/// Example of how to convert paths to asset-style paths (like the Python _to_asset_style_path function)
pub fn to_asset_style_path(path: &Path, base_dir: &Path) -> String {
    // Look for "Content" directory in the path
    if let Some(content_idx) = path.components().position(|comp| {
        comp.as_os_str().to_string_lossy().to_lowercase() == "content"
    }) {
        if content_idx > 0 {
            return path.components()
                .skip(content_idx - 1)
                .collect::<PathBuf>()
                .to_string_lossy()
                .replace("\\", "/");
        }
    }

    // Fallback to relative path from base directory
    if let Ok(rel_path) = path.strip_prefix(base_dir) {
        return rel_path.to_string_lossy().replace("\\", "/");
    }

    // Final fallback to just the filename
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}