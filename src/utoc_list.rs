//! UTOC file listing functionality
//!
//! This module provides programmatic access to listing Unreal Engine .utoc files
//! using the retoc-rivals library.

use std::path::Path;
use std::sync::Arc;
use std::io::{Read, BufReader, Write};
use std::fs::File;

use crate::error::{Result, UeToolError};
use crate::types::{AssetPath, UtocListOptions, FileEntry, CompressionMethod, ProgressInfo, ProgressCallback};

use serde::{Deserialize, Serialize};
use fs_err as fs;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;

/// UTOC file chunk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtocChunkInfo {
    pub id: String,
    pub path: Option<String>,
    pub size: u64,
    pub compressed_size: u64,
    pub offset: u64,
    pub chunk_type: String,
    pub is_compressed: bool,
    pub compression_method: Option<CompressionMethod>,
}

/// UTOC file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtocMetadata {
    pub version: u32,
    pub container_id: String,
    pub file_count: u32,
    pub chunk_count: u32,
    pub compression_methods: Vec<String>,
    pub container_flags: u32,
}

/// Main struct for listing .utoc file contents
pub struct UtocLister {
    progress_callback: Option<ProgressCallback>,
}

impl UtocLister {
    /// Create a new .utoc lister
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

    /// List contents of a .utoc file
    pub fn list<P: AsRef<Path>>(
        &mut self,
        utoc_path: P,
        options: &UtocListOptions,
    ) -> Result<Vec<AssetPath>> {
        let utoc_path = utoc_path.as_ref();

        if !utoc_path.exists() {
            return Err(UeToolError::file_not_found(utoc_path));
        }

        self.report_progress(ProgressInfo {
            percentage: 0,
            message: "Opening UTOC file".to_string(),
            processed: 0,
            total: 1,
        });

        // Use retoc library for proper UTOC parsing
        let config = retoc::Config {
            aes_keys: if let Some(ref aes_key) = options.aes_key {
                let mut keys = HashMap::new();
                keys.insert(retoc::FGuid::default(), aes_key.parse().map_err(|e| UeToolError::InvalidAesKey(format!("Invalid AES key: {}", e)))?);
                keys
            } else {
                HashMap::new()
            },
            container_header_version_override: None,
        };

        let config = Arc::new(config);

        self.report_progress(ProgressInfo {
            percentage: 10,
            message: "Parsing UTOC structure".to_string(),
            processed: 0,
            total: 1,
        });

        // Open UTOC file using retoc
        let iostore = retoc::open_iostore(utoc_path, config.clone())
            .map_err(|e| UeToolError::UtocError(format!("Failed to open UTOC file: {}", e)))?;

        self.report_progress(ProgressInfo {
            percentage: 30,
            message: "Extracting file listing".to_string(),
            processed: 0,
            total: 1,
        });

        let chunks: Vec<_> = iostore.chunks().collect();

        self.report_progress(ProgressInfo {
            percentage: 60,
            message: format!("Found {} chunks", chunks.len()),
            processed: 0,
            total: chunks.len() as u64,
        });

        let mut asset_paths = Vec::new();
        let mut processed = 0;

        for chunk in &chunks {
            processed += 1;
            let progress_percentage = 60 + ((processed as f64 / chunks.len() as f64) * 35.0) as u8;
            
            self.report_progress(ProgressInfo {
                percentage: progress_percentage,
                message: format!("Processing chunk {}", processed),
                processed: processed as u64,
                total: chunks.len() as u64,
            });

            // Get chunk information
            let chunk_id = chunk.id();
            let chunk_path = chunk.path();
            
            // Only include chunks that have file paths and are asset files
            if let Some(ref path) = chunk_path {
                if self.is_asset_file(path) {
                    asset_paths.push(AssetPath::new(path.to_string()));
                }
            }
        }

        asset_paths.sort();
        asset_paths.dedup();

        self.report_progress(ProgressInfo {
            percentage: 100,
            message: format!("Completed - found {} assets", asset_paths.len()),
            processed: processed as u64,
            total: processed as u64,
        });

        if !options.json_format && !options.aes_key.is_some() {
            // Simple console output for non-JSON mode
            println!("Found {} assets in {}", asset_paths.len(), utoc_path.display());
            for asset in &asset_paths {
                println!("  {}", asset.as_str());
            }
        }

        Ok(asset_paths)
    }

    /// Get detailed information about UTOC file contents (JSON output)
    pub fn list_detailed<P: AsRef<Path>>(
        &mut self,
        utoc_path: P,
        options: &UtocListOptions,
    ) -> Result<UtocFileInfo> {
        let utoc_path = utoc_path.as_ref();

        if !utoc_path.exists() {
            return Err(UeToolError::file_not_found(utoc_path));
        }

        // Use retoc library for proper UTOC parsing
        let config = retoc::Config {
            aes_keys: if let Some(ref aes_key) = options.aes_key {
                let mut keys = HashMap::new();
                keys.insert(retoc::FGuid::default(), aes_key.parse().map_err(|e| UeToolError::InvalidAesKey(format!("Invalid AES key: {}", e)))?);
                keys
            } else {
                HashMap::new()
            },
            container_header_version_override: None,
        };

        let config = Arc::new(config);

        // Open UTOC file using retoc
        let iostore = retoc::open_iostore(utoc_path, config)
            .map_err(|e| UeToolError::UtocError(format!("Failed to open UTOC file: {}", e)))?;

        let chunks: Vec<_> = iostore.chunks().collect();
        let metadata = UtocMetadata {
            version: iostore.container_file_version().unwrap_or_default() as u32,
            container_id: format!("{:?}", iostore.container_name()),
            file_count: chunks.len() as u32,
            chunk_count: chunks.len() as u32,
            compression_methods: vec![], // TODO: extract compression methods
            container_flags: 0, // TODO: extract container flags
        };

        let mut file_entries = Vec::new();
        let mut asset_paths = Vec::new();

        for chunk in chunks {
            let chunk_id = chunk.id();
            let chunk_path = chunk.path();
            let chunk_type = chunk.id().get_chunk_type();
            
            if let Some(ref path) = chunk_path {
                if self.is_asset_file(path) {
                    asset_paths.push(AssetPath::new(path.to_string()));
                    
                    file_entries.push(FileEntry {
                        path: AssetPath::new(path.to_string()),
                        size: 0, // TODO: get actual size
                        is_compressed: false, // TODO: determine compression
                        compression: None, // TODO: extract compression method
                    });
                }
            }
        }

        asset_paths.sort();
        asset_paths.dedup();

        Ok(UtocFileInfo {
            metadata,
            assets: asset_paths,
            file_entries,
        })
    }

    /// Check if a path represents an asset file
    fn is_asset_file(&self, path: &str) -> bool {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        matches!(ext.as_str(),
            "uasset" | "umap" | "bnk" | "json" | "wem" | "fbx" | "obj" | "glb" | "gltf" |
            "ini" | "wav" | "mp3" | "ogg" | "uplugin" | "usf"
        )
    }

    /// Report progress to callback if set
    fn report_progress(&mut self, progress: ProgressInfo) {
        if let Some(ref mut callback) = self.progress_callback {
            callback(progress);
        }
    }
}

/// Complete UTOC file information for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtocFileInfo {
    pub metadata: UtocMetadata,
    pub assets: Vec<AssetPath>,
    pub file_entries: Vec<FileEntry>,
}

impl Default for UtocLister {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utoc_lister_creation() {
        let lister = UtocLister::new();
        assert!(lister.progress_callback.is_none());
    }

    #[test]
    fn test_progress_callback() {
        let callback = Box::new(|_: ProgressInfo| {});
        let lister = UtocLister::new().with_progress_callback(callback);
        assert!(lister.progress_callback.is_some());
    }
}