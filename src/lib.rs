//! Unified Rust library for Unreal Engine pak and.utoc operations
//! 
//! This library provides programmatic access to repak and retoc_cli functionality
//! without needing external command-line tools.
//!
//! ## Features
//!
//! - Unpack .pak files (similar to `repak unpack`)
//! - List contents of .utoc files (similar to `retoc_cli list`)
//! - Support for AES encrypted files
//! - Compression support (Oodle, Zstd, Zlib, etc.)
//! - Archive support (ZIP and RAR files)
//! - Progress reporting for long operations
//!

use std::path::{Path, PathBuf};
use std::io::{BufReader, BufWriter};
use std::fs::{self, File};
use std::process::Command;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod pak_unpack;
pub mod utoc_list;
pub mod error;
pub mod types;
pub mod cli;
pub mod python_bindings;

pub use error::{Result, UeToolError};
pub use pak_unpack::PakUnpacker;
pub use utoc_list::UtocLister;
pub use types::{AssetPath, CompressionMethod, PakUnpackOptions, UtocListOptions};

// Re-export common types for convenience
pub use rayon::prelude::*;

/// Main entry point for unpacking pak files
pub struct Unpacker {
    pub pak_unpacker: PakUnpacker,
    pub utoc_lister: UtocLister,
}

impl Unpacker {
    /// Create a new unpacker instance
    pub fn new() -> Self {
        Self {
            pak_unpacker: PakUnpacker::new(),
            utoc_lister: UtocLister::new(),
        }
    }

    /// Unpack a pak file to the specified output directory
    ///
    /// # Arguments
    /// * `pak_path` - Path to the .pak file to unpack
    /// * `output_dir` - Directory where files should be extracted
    /// * `options` - Unpack options (aes key, compression, etc.)
    pub fn unpack_pak<P: AsRef<Path>>(
        &mut self,
        pak_path: P,
        output_dir: P,
        options: &PakUnpackOptions,
    ) -> Result<Vec<AssetPath>> {
        let unpacked_files = self.pak_unpacker.unpack(pak_path, output_dir, options)?;
        Ok(unpacked_files.into_iter().map(|f| f.original_path).collect())
    }

    /// List contents of a.utoc file
    ///
    /// # Arguments
    /// * `utoc_path` - Path to the.utoc file to list
    /// * `options` - List options (aes key, format, etc.)
    pub fn list_utoc<P: AsRef<Path>>(
        &mut self,
        utoc_path: P,
        options: &UtocListOptions,
    ) -> Result<Vec<AssetPath>> {
        self.utoc_lister.list(utoc_path, options)
    }

    /// Get file list from a pak file without reading content (for solo pak files)
    ///
    /// # Arguments
    /// * `pak_path` - Path to the .pak file
    /// * `aes_key` - Optional AES key for encrypted files
    pub fn get_pak_file_list<P: AsRef<Path>>(
        &mut self,
        pak_path: P,
        aes_key: Option<&str>,
    ) -> Result<Vec<AssetPath>> {
        let pak_path = pak_path.as_ref();

        if !pak_path.exists() {
            return Err(UeToolError::file_not_found(pak_path));
        }

        // Create PakBuilder and open the pak file
        let mut builder = repak::PakBuilder::new();
        if let Some(ref key) = aes_key {
            if let Ok(aes_key) = key.parse::<repak::utils::AesKey>() {
                builder = builder.key(aes_key.0);
            } else {
                return Err(UeToolError::InvalidAesKey(format!("Invalid AES key format: {}", key)));
            }
        }

        let pak_file = File::open(pak_path)
            .map_err(|e| UeToolError::IoError(format!("Failed to open PAK file: {}", e)))?;
        let mut reader = BufReader::new(pak_file);
        
        let pak = builder.reader(&mut reader)
            .map_err(|e| UeToolError::PakError(format!("Failed to read PAK file: {}", e)))?;

        // Just return the file list without reading content
        let files = pak.files();
        Ok(files.into_iter().map(|path| AssetPath::new(path)).collect())
    }

    /// Extract asset paths from an archive file (ZIP or RAR) containing pak/utoc files
    ///
    /// This function will:
    /// 1. Detect archive type (ZIP or RAR)
    /// 2. Extract the archive to a temporary directory
    /// 3. Find and unpack any .pak files
    /// 4. List contents of any .utoc files
    /// 5. Return all discovered asset paths
    pub fn extract_asset_paths_from_archive<P: AsRef<Path>>(
        &mut self,
        archive_path: P,
        aes_key: Option<&str>,
        keep_temp: bool,
    ) -> Result<Vec<AssetPath>> {
        use tempfile::TempDir;
        use walkdir::WalkDir;

        // Create temporary directory
        let temp_dir = TempDir::new()
            .map_err(|e| UeToolError::IoError(format!("Failed to create temp directory: {}", e)))?;
        let temp_path = temp_dir.path().to_path_buf();

        // Detect archive type and extract
        let archive_path = archive_path.as_ref();
        let archive_type = Self::detect_archive_type(archive_path)?;

        match archive_type {
            ArchiveType::Zip => {
                self.extract_zip_archive(archive_path, &temp_path)?;
            }
            ArchiveType::Rar => {
                self.extract_rar_archive(archive_path, &temp_path)?;
            }
        }

        let mut all_assets = Vec::new();

        // Process extracted UE files (pak and utoc)
        println!("Processing extracted UE files:");

        // Find all pak and utoc files
        let pak_files: Vec<PathBuf> = WalkDir::new(&temp_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "pak"))
            .map(|entry| entry.path().to_path_buf())
            .collect();

        let utoc_files: Vec<PathBuf> = WalkDir::new(&temp_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "utoc"))
            .map(|entry| entry.path().to_path_buf())
            .collect();

        // Create a set of pak file names without extension for matching
        let mut pak_names = std::collections::HashSet::new();
        for pak_file in &pak_files {
            if let Some(name) = pak_file.file_stem() {
                pak_names.insert(name.to_string_lossy().to_string());
            }
        }

        // Process pak files - handle both solo pak and bundle pak
        for pak_file in &pak_files {
            let pak_name = pak_file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            // Check if this pak has a corresponding utoc file (bundle scenario)
            let has_utoc = utoc_files.iter().any(|utoc| {
                utoc.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|utoc_name| utoc_name == pak_name)
                    .unwrap_or(false)
            });

            if has_utoc {
                // Bundle scenario: Skip pak file processing since we'll get assets from utoc
                println!("Skipping pak file {} (has corresponding utoc)", pak_file.display());
                continue;
            }

            // Solo pak scenario: Get file list without reading content
            println!("Processing solo pak file: {}", pak_file.display());
            match self.get_pak_file_list(pak_file, aes_key) {
                Ok(assets) => {
                    println!("  Found {} assets in solo pak file", assets.len());
                    all_assets.extend(assets);
                }
                Err(e) => println!("Warning: Failed to read pak file {}: {}", pak_file.display(), e),
            }
        }

        // Process utoc files (for bundles)
        let utoc_options = UtocListOptions {
            aes_key: aes_key.map(|s| s.to_string()),
            json_format: false,
        };

        for utoc_file in &utoc_files {
            println!("Processing utoc file: {}", utoc_file.display());
            match self.list_utoc(utoc_file, &utoc_options) {
                Ok(assets) => {
                    println!("  Found {} assets in utoc file", assets.len());
                    all_assets.extend(assets);
                }
                Err(e) => println!("Warning: Failed to list utoc file {}: {}", utoc_file.display(), e),
            }
        }

        if keep_temp {
            // Return temp dir so caller can access it
            drop(temp_dir);
        }

        Ok(all_assets)
    }

    /// Detect the type of archive file
    fn detect_archive_type(archive_path: &Path) -> Result<ArchiveType> {
        let extension = archive_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => Ok(ArchiveType::Zip),
            "rar" => Ok(ArchiveType::Rar),
            _ => Err(UeToolError::InvalidArgument(format!(
                "Unsupported archive type: {}", extension
            ))),
        }
    }

    /// Extract a ZIP archive to the specified directory
    fn extract_zip_archive(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let zip_file = File::open(archive_path)
            .map_err(|e| UeToolError::IoError(format!("Failed to open zip file: {}", e)))?;

        let mut zip_archive = zip::ZipArchive::new(zip_file)
            .map_err(|e| UeToolError::IoError(format!("Failed to open zip archive: {}", e)))?;

        for i in 0..zip_archive.len() {
            let mut file = zip_archive.by_index(i)
                .map_err(|e| UeToolError::IoError(format!("Failed to read zip entry {}: {}", i, e)))?;

            let out_path = dest_dir.join(file.name());
            if file.is_dir() {
                fs::create_dir_all(&out_path)
                    .map_err(|e| UeToolError::IoError(format!("Failed to create directory {}: {}", out_path.display(), e)))?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| UeToolError::IoError(format!("Failed to create parent directory {}: {}", parent.display(), e)))?;
                }
                let mut out_file = File::create(&out_path)
                    .map_err(|e| UeToolError::IoError(format!("Failed to create file {}: {}", out_path.display(), e)))?;
                std::io::copy(&mut file, &mut out_file)
                    .map_err(|e| UeToolError::IoError(format!("Failed to copy file data: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Extract a RAR archive to the specified directory using external tools
    fn extract_rar_archive(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        use std::process::Command;

        // Try to find RAR tool (similar to Python implementation)
        let rar_tool = Self::find_rar_tool()?;

        // Run the RAR extraction command
        let output = Command::new(&rar_tool)
            .args(&["x", "-y"]) // x = extract, -y = assume yes to all prompts
            .arg(archive_path)
            .arg(dest_dir)
            .output()
            .map_err(|e| UeToolError::IoError(format!("Failed to run RAR tool: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(UeToolError::IoError(format!(
                "RAR extraction failed: {}", stderr
            )));
        }

        Ok(())
    }

    /// Find the RAR tool executable (similar to Python implementation)
    fn find_rar_tool() -> Result<String> {
        // Check environment variable first
        if let Ok(env_tool) = std::env::var("RAR_TOOL_PATH") {
            if Path::new(&env_tool).exists() {
                return Ok(env_tool);
            }
        }

        // Check common WinRAR locations
        let winrar_paths = [
            r"C:\Program Files\WinRAR\rar.exe",
            r"C:\Program Files (x86)\WinRAR\rar.exe",
            r"C:\WinRAR\rar.exe",
        ];

        for path in &winrar_paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }

        // Check if rar.exe is in PATH
        if let Ok(output) = Command::new("where").arg("rar.exe").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }

        Err(UeToolError::IoError("No RAR tool found. Please install WinRAR or ensure rar.exe is in PATH".to_string()))
    }
}

/// Archive type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveType {
    Zip,
    Rar,
}

impl Default for Unpacker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpacker_creation() {
        let unpacker = Unpacker::new();
        // Just verify it can be created
        assert!(!std::ptr::null(&unpacker.pak_unpacker));
        assert!(!std::ptr::null(&unpacker.utoc_lister));
    }
}