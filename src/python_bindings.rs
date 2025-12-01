//! Python bindings for the rust-ue-tools library
//!
//! This module provides PyO3 bindings for native Python integration.
//! PyO3 is now the only supported binding method.

use pyo3::prelude::*;
use std::collections::HashMap;
use tempfile;
use crate::{Unpacker, AssetPath};
use crate::error::{UeToolError, Result};

#[pyclass]
pub struct PyAssetPath {
    asset_path: AssetPath,
}

#[pymethods]
impl PyAssetPath {
    #[new]
    fn new(path: String) -> Self {
        Self {
            asset_path: AssetPath::new(path),
        }
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(self.asset_path.0.clone())
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("AssetPath('{}')", self.asset_path.0))
    }

    #[getter]
    fn path(&self) -> PyResult<String> {
        Ok(self.asset_path.0.clone())
    }

    fn has_extension(&self, ext: &str) -> bool {
        self.asset_path.has_extension(ext)
    }

    fn extension(&self) -> Option<String> {
        self.asset_path.extension().map(|s| s.to_string())
    }

    fn file_name(&self) -> Option<String> {
        self.asset_path.file_name().map(|s| s.to_string())
    }
}

#[pyclass]
pub struct PyUnpacker {
    unpacker: Unpacker,
}

#[pymethods]
impl PyUnpacker {
    #[new]
    fn new() -> Self {
        Self {
            unpacker: Unpacker::new(),
        }
    }

    #[pyo3(signature = (zip_path, aes_key = None, keep_temp = false))]
    fn extract_asset_paths_from_zip(
        &mut self,
        zip_path: &str,
        aes_key: Option<&str>,
        keep_temp: bool,
    ) -> PyResult<Vec<PyAssetPath>> {
        match self.unpacker.extract_asset_paths_from_archive(zip_path, aes_key, keep_temp) {
            Ok(assets) => Ok(assets.into_iter().map(|a| PyAssetPath { asset_path: a }).collect()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyIOError, String>(e.to_string())),
        }
    }

    #[pyo3(signature = (folder_path, aes_key = None))]
    fn extract_pak_asset_map_from_folder(
        &mut self,
        folder_path: &str,
        aes_key: Option<&str>,
    ) -> PyResult<HashMap<String, Vec<String>>> {
        // Implement folder scanning logic
        use walkdir::WalkDir;

        let mut result_map: HashMap<String, Vec<String>> = HashMap::new();

        // Find all pak and utoc files
        eprintln!("[DEBUG] Scanning folder: {}", folder_path);
        let mut pak_files = Vec::new();
        let mut utoc_files = Vec::new();
        
        for entry in WalkDir::new(folder_path) {
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    eprintln!("[DEBUG] Found file: {}", entry.path().display());
                    if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                        eprintln!("[DEBUG]   Extension: {}", ext);
                        if ext.eq_ignore_ascii_case("pak") {
                            eprintln!("[DEBUG]   -> Adding to pak_files");
                            pak_files.push(entry.path().to_path_buf());
                        } else if ext.eq_ignore_ascii_case("utoc") {
                            eprintln!("[DEBUG]   -> Adding to utoc_files");
                            utoc_files.push(entry.path().to_path_buf());
                        }
                    }
                }
                _ => {}
            }
        }

        eprintln!("[DEBUG] Found {} pak files and {} utoc files", pak_files.len(), utoc_files.len());
        // Process pak files
        for pak_path in &pak_files {
            let pak_name = pak_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            // Check if this pak has a corresponding utoc file (bundle scenario)
            let has_utoc = utoc_files.iter().any(|utoc| {
                utoc.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|utoc_name| utoc_name == pak_name)
                    .unwrap_or(false)
            });

            if has_utoc {
                // Bundle scenario: will be handled by utoc processing
                continue;
            }

            // Solo pak scenario: unpack to get file list (like basic_usage.rs example)
            let temp_dir = tempfile::tempdir()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, String>(
                    format!("Failed to create temp directory: {}", e)
                ))?;
            let temp_path = temp_dir.path().to_path_buf();

            let options = crate::PakUnpackOptions {
                aes_key: aes_key.map(|s| s.to_string()),
                strip_prefix: "../../../".to_string(),
                force: true,
                quiet: true,
                include_patterns: Vec::new(),
            };

            match self.unpacker.unpack_pak(pak_path, &temp_path, &options) {
                Ok(assets) => {
                    let asset_paths: Vec<String> = assets.into_iter().map(|a| a.0).collect();
                    if !asset_paths.is_empty() {
                        result_map.insert(format!("{}.pak", pak_name), asset_paths);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to read pak file {}: {}", pak_path.display(), e);
                }
            }
        }

        // Process utoc files
        let utoc_options = crate::UtocListOptions {
            aes_key: aes_key.map(|s| s.to_string()),
            json_format: false,
        };

        eprintln!("[DEBUG] Processing {} utoc files", utoc_files.len());
        for utoc_path in &utoc_files {
            let utoc_name = utoc_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            eprintln!("[DEBUG] Processing UTOC: {}", utoc_path.display());
            eprintln!("[DEBUG]   File exists: {}", utoc_path.exists());
            eprintln!("[DEBUG]   File size: {:?}", utoc_path.metadata().map(|m| m.len()));
            eprintln!("[DEBUG]   AES key available: {}", aes_key.is_some());

            match self.unpacker.list_utoc(utoc_path, &utoc_options) {
                Ok(assets) => {
                    eprintln!("[DEBUG]   UTOC parse SUCCESS: Found {} assets", assets.len());
                    let asset_paths: Vec<String> = assets.into_iter().map(|a| a.0).collect();
                    
                    if !asset_paths.is_empty() {
                        eprintln!("[DEBUG]   Inserting {}.utoc with {} assets", utoc_name, asset_paths.len());
                        result_map.insert(format!("{}.utoc", utoc_name), asset_paths);
                    } else {
                        // FALLBACK: If UTOC returns 0 assets, try reading from the PAK file instead
                        eprintln!("[DEBUG]   WARNING: UTOC returned 0 assets, falling back to PAK file");
                        
                        // Find corresponding PAK file
                        let pak_path = utoc_path.with_extension("pak");
                        if pak_path.exists() {
                            eprintln!("[DEBUG]   Found corresponding PAK file: {}", pak_path.display());
                            
                            match self.unpacker.get_pak_file_list(&pak_path, aes_key) {
                                Ok(pak_assets) => {
                                    eprintln!("[DEBUG]   PAK fallback SUCCESS: Found {} assets", pak_assets.len());
                                    let pak_asset_paths: Vec<String> = pak_assets.into_iter().map(|a| a.0).collect();
                                    
                                    if !pak_asset_paths.is_empty() {
                                        eprintln!("[DEBUG]   Inserting {}.utoc (from PAK) with {} assets", utoc_name, pak_asset_paths.len());
                                        result_map.insert(format!("{}.utoc", utoc_name), pak_asset_paths);
                                    } else {
                                        eprintln!("[DEBUG]   WARNING: PAK file also returned 0 assets");
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[ERROR] PAK fallback failed for {}: {}", pak_path.display(), e);
                                }
                            }
                        } else {
                            eprintln!("[DEBUG]   No corresponding PAK file found at: {}", pak_path.display());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to list utoc file {}: {}", utoc_path.display(), e);
                    eprintln!("[ERROR]   Error details: {:?}", e);
                }
            }
        }

        Ok(result_map)
    }

    #[pyo3(signature = (pak_path, output_dir, aes_key = None, force = false, quiet = true))]
    fn unpack_pak(
        &mut self,
        pak_path: &str,
        output_dir: &str,
        aes_key: Option<&str>,
        force: bool,
        quiet: bool,
    ) -> PyResult<Vec<PyAssetPath>> {
        let options = crate::PakUnpackOptions {
            aes_key: aes_key.map(|s| s.to_string()),
            strip_prefix: "../../../".to_string(),
            force,
            quiet,
            include_patterns: Vec::new(),
        };

        match self.unpacker.unpack_pak(pak_path, output_dir, &options) {
            Ok(assets) => Ok(assets.into_iter().map(|a| PyAssetPath { asset_path: a }).collect()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyIOError, String>(e.to_string())),
        }
    }

    #[pyo3(signature = (utoc_path, aes_key = None, json_format = false))]
    fn list_utoc(&mut self, utoc_path: &str, aes_key: Option<&str>, json_format: bool) -> PyResult<Vec<PyAssetPath>> {
        let options = crate::UtocListOptions {
            aes_key: aes_key.map(|s| s.to_string()),
            json_format,
        };

        match self.unpacker.list_utoc(utoc_path, &options) {
            Ok(assets) => Ok(assets.into_iter().map(|a| PyAssetPath { asset_path: a }).collect()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyIOError, String>(e.to_string())),
        }
    }

    #[pyo3(signature = (pak_path, aes_key = None))]
    fn get_pak_file_list(&mut self, pak_path: &str, aes_key: Option<&str>) -> PyResult<Vec<PyAssetPath>> {
        match self.unpacker.get_pak_file_list(pak_path, aes_key) {
            Ok(assets) => Ok(assets.into_iter().map(|a| PyAssetPath { asset_path: a }).collect()),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyIOError, String>(e.to_string())),
        }
    }
}

#[pymodule]
fn rust_ue_tools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyAssetPath>()?;
    m.add_class::<PyUnpacker>()?;
    Ok(())
}
