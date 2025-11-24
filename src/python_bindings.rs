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
        let mut pak_files = Vec::new();
        let mut utoc_files = Vec::new();
        
        for entry in WalkDir::new(folder_path) {
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                        if ext == "pak" {
                            pak_files.push(entry.path().to_path_buf());
                        } else if ext == "utoc" {
                            utoc_files.push(entry.path().to_path_buf());
                        }
                    }
                }
                _ => {}
            }
        }

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
                        result_map.insert(pak_name.to_string(), asset_paths);
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

        for utoc_path in &utoc_files {
            let utoc_name = utoc_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            match self.unpacker.list_utoc(utoc_path, &utoc_options) {
                Ok(assets) => {
                    let asset_paths: Vec<String> = assets.into_iter().map(|a| a.0).collect();
                    if !asset_paths.is_empty() {
                        result_map.insert(format!("{}.utoc", utoc_name), asset_paths);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to list utoc file {}: {}", utoc_path.display(), e);
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