//! PAK file unpacking functionality
//!
//! This module provides programmatic access to unpacking Unreal Engine .pak files
//! using the repak library.

use std::path::Path;
use std::io::BufReader;
use std::fs::File;

use crate::error::{Result, UeToolError};
use crate::types::{AssetPath, PakUnpackOptions, UnpackedFile, ProgressInfo, ProgressCallback};

/// Main struct for unpacking pak files
pub struct PakUnpacker {
    progress_callback: Option<ProgressCallback>,
}

impl PakUnpacker {
    /// Create a new pak unpacker
    pub fn new() -> Self {
        Self {
            progress_callback: None,
        }
    }

    /// Set progress callback for long operations
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Unpack a pak file to the specified output directory
    pub fn unpack<P: AsRef<Path>>(
        &mut self,
        pak_path: P,
        output_dir: P,
        options: &PakUnpackOptions,
    ) -> Result<Vec<UnpackedFile>> {
        let pak_path = pak_path.as_ref();
        let output_dir = output_dir.as_ref();

        if !pak_path.exists() {
            return Err(UeToolError::file_not_found(pak_path));
        }

        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)
                .map_err(|e| UeToolError::IoError(format!("Failed to create output directory: {}", e)))?;
        }

        self.report_progress(ProgressInfo {
            percentage: 0,
            message: "Opening PAK file".to_string(),
            processed: 0,
            total: 1,
        });

        // Create PakBuilder and open the pak file
        let mut builder = repak::PakBuilder::new();
        if let Some(ref aes_key) = options.aes_key {
            if let Ok(key) = aes_key.parse::<repak::utils::AesKey>() {
                builder = builder.key(key.0);
            } else {
                return Err(UeToolError::InvalidAesKey(format!("Invalid AES key format: {}", aes_key)));
            }
        }

        let pak_file = File::open(pak_path)
            .map_err(|e| UeToolError::IoError(format!("Failed to open PAK file: {}", e)))?;
        let mut reader = BufReader::new(pak_file);
        let pak = builder.reader(&mut reader)
            .map_err(|e| UeToolError::PakError(format!("Failed to read PAK file: {}", e)))?;

        self.report_progress(ProgressInfo {
            percentage: 20,
            message: "Reading file list".to_string(),
            processed: 0,
            total: 1,
        });

        let files = pak.files();
        let total_files = files.len();

        self.report_progress(ProgressInfo {
            percentage: 30,
            message: format!("Found {} files to unpack", total_files),
            processed: 0,
            total: total_files as u64,
        });

        let mut unpacked_files = Vec::new();
        let mut processed = 0;

        // Process each file
        for file_path in files {
            processed += 1;
            let progress_percentage = 30 + ((processed as f64 / total_files as f64) * 70.0) as u8;

            self.report_progress(ProgressInfo {
                percentage: progress_percentage,
                message: format!("Unpacking: {}", file_path),
                processed: processed as u64,
                total: total_files as u64,
            });

            // Apply strip prefix if specified
            let stripped_path = if !options.strip_prefix.is_empty() {
                file_path.strip_prefix(&options.strip_prefix).unwrap_or(&file_path)
            } else {
                &file_path
            };

            // Create output path
            let output_path = output_dir.join(stripped_path);

            // Create parent directories
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| UeToolError::IoError(format!("Failed to create directory: {}", e)))?;
            }

            // Read file data
            let data = pak.get(&file_path, &mut reader)
                .map_err(|e| UeToolError::PakError(format!("Failed to read file {}: {}", file_path, e)))?;

            // Write file
            std::fs::write(&output_path, &data)
                .map_err(|e| UeToolError::IoError(format!("Failed to write file {}: {}", output_path.display(), e)))?;

            // Create UnpackedFile info
            let unpacked_file = UnpackedFile {
                original_path: AssetPath::new(file_path.clone()),
                output_path: output_path.clone(),
                size: data.len() as u64,
                error: None,
            };

            unpacked_files.push(unpacked_file);
        }

        self.report_progress(ProgressInfo {
            percentage: 100,
            message: format!("Completed - unpacked {} files", unpacked_files.len()),
            processed: processed as u64,
            total: processed as u64,
        });

        Ok(unpacked_files)
    }

    /// Report progress to callback if set
    fn report_progress(&mut self, progress: ProgressInfo) {
        if let Some(ref mut callback) = self.progress_callback {
            callback(progress);
        }
    }

    /// List files in a pak file without extracting them
    pub fn list_files<P: AsRef<Path>>(
        &mut self,
        pak_path: P,
        _options: &PakUnpackOptions,
    ) -> Result<Vec<AssetPath>> {
        let pak_path = pak_path.as_ref();

        if !pak_path.exists() {
            return Err(UeToolError::file_not_found(pak_path));
        }

        // Create PakBuilder and open the pak file
        let mut builder = repak::PakBuilder::new();

        let pak_file = File::open(pak_path)
            .map_err(|e| UeToolError::IoError(format!("Failed to open PAK file: {}", e)))?;
        let mut reader = BufReader::new(pak_file);
        
        let pak = builder.reader(&mut reader)
            .map_err(|e| UeToolError::PakError(format!("Failed to read PAK file: {}", e)))?;

        // Just return the file list without reading content
        let files = pak.files();
        Ok(files.into_iter().map(|path| AssetPath::new(path)).collect())
    }

    /// Get information about a pak file
    pub fn get_info<P: AsRef<Path>>(
        &mut self,
        pak_path: P,
        _options: &PakUnpackOptions,
    ) -> Result<serde_json::Value> {
        let pak_path = pak_path.as_ref();

        if !pak_path.exists() {
            return Err(UeToolError::file_not_found(pak_path));
        }

        // Create PakBuilder and open the pak file
        let mut builder = repak::PakBuilder::new();

        let pak_file = File::open(pak_path)
            .map_err(|e| UeToolError::IoError(format!("Failed to open PAK file: {}", e)))?;
        let mut reader = BufReader::new(pak_file);
        
        let pak = builder.reader(&mut reader)
            .map_err(|e| UeToolError::PakError(format!("Failed to read PAK file: {}", e)))?;

        let files = pak.files();
        let file_count = files.len();

        let info = serde_json::json!({
            "file_path": pak_path.to_string_lossy(),
            "file_count": file_count,
            "file_names": files
        });

        Ok(info)
    }
}

impl Default for PakUnpacker {
    fn default() -> Self {
        Self::new()
    }
}