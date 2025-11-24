//! Common types and data structures for the UE tools library

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use glob::Pattern;

/// Represents a UE asset path (like "/Game/Characters/Hero/Hero.uasset")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct AssetPath(pub String);

impl AssetPath {
    /// Create a new asset path
    pub fn new<S: Into<String>>(path: S) -> Self {
        Self(path.into())
    }

    /// Get the path as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this asset path has a specific extension
    pub fn has_extension<S: AsRef<str>>(&self, ext: S) -> bool {
        self.0.ends_with(&format!(".{}", ext.as_ref()))
    }

    /// Get the file extension (without the dot)
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.0).extension().and_then(|s| s.to_str())
    }

    /// Get the file name part of the path
    pub fn file_name(&self) -> Option<&str> {
        Path::new(&self.0).file_name().and_then(|s| s.to_str())
    }

    /// Get the parent directory path
    pub fn parent(&self) -> Option<AssetPath> {
        Path::new(&self.0).parent().map(|p| AssetPath(p.to_string_lossy().to_string()))
    }

    /// Check if this path starts with another path
    pub fn starts_with<P: AsRef<Path>>(&self, prefix: P) -> bool {
        self.0.starts_with(&prefix.as_ref().as_os_str().to_string_lossy().to_string())
    }
}

impl From<String> for AssetPath {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AssetPath {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<PathBuf> for AssetPath {
    fn from(path: PathBuf) -> Self {
        Self(path.to_string_lossy().to_string())
    }
}

impl From<AssetPath> for String {
    fn from(asset_path: AssetPath) -> Self {
        asset_path.0
    }
}

impl std::fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Supported compression methods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionMethod {
    /// No compression
    None,
    /// Zlib compression
    Zlib,
    /// Gzip compression
    Gzip,
    /// Oodle compression (default)
    Oodle,
    /// Zstd compression
    Zstd,
    /// LZ4 compression
    Lz4,
}

impl Default for CompressionMethod {
    fn default() -> Self {
        CompressionMethod::Oodle
    }
}

impl FromStr for CompressionMethod {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" | "no" | "uncompressed" => Ok(CompressionMethod::None),
            "zlib" | "deflate" => Ok(CompressionMethod::Zlib),
            "gzip" => Ok(CompressionMethod::Gzip),
            "oodle" | "oodle2" => Ok(CompressionMethod::Oodle),
            "zstd" | "zstandard" => Ok(CompressionMethod::Zstd),
            "lz4" => Ok(CompressionMethod::Lz4),
            _ => Err(format!("Unknown compression method: {}", s)),
        }
    }
}

impl std::fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionMethod::None => write!(f, "None"),
            CompressionMethod::Zlib => write!(f, "Zlib"),
            CompressionMethod::Gzip => write!(f, "Gzip"),
            CompressionMethod::Oodle => write!(f, "Oodle"),
            CompressionMethod::Zstd => write!(f, "Zstd"),
            CompressionMethod::Lz4 => write!(f, "Lz4"),
        }
    }
}

/// Options for unpacking pak files
#[derive(Debug, Clone)]
pub struct PakUnpackOptions {
    pub aes_key: Option<String>,
    pub strip_prefix: String,
    pub force: bool,
    pub quiet: bool,
    pub include_patterns: Vec<Pattern>,
}

impl Default for PakUnpackOptions {
    fn default() -> Self {
        Self {
            aes_key: None,
            strip_prefix: "../../../".to_string(),
            force: false,
            quiet: false,
            include_patterns: vec![],
        }
    }
}

impl PakUnpackOptions {
    /// Create new options with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the AES key for encrypted files
    pub fn with_aes_key<S: Into<String>>(mut self, key: S) -> Self {
        self.aes_key = Some(key.into());
        self
    }

    /// Set the strip prefix for paths
    pub fn with_strip_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.strip_prefix = prefix.into();
        self
    }

    /// Enable force mode (overwrite existing files)
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Enable quiet mode (minimal output)
    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Add include patterns for file filtering
    pub fn with_include_patterns(mut self, patterns: Vec<Pattern>) -> Self {
        self.include_patterns = patterns;
        self
    }
}

/// Options for listing.utoc files
#[derive(Debug, Clone)]
pub struct UtocListOptions {
    pub aes_key: Option<String>,
    pub json_format: bool,
}

impl Default for UtocListOptions {
    fn default() -> Self {
        Self {
            aes_key: None,
            json_format: false,
        }
    }
}

impl UtocListOptions {
    /// Create new options with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the AES key for encrypted files
    pub fn with_aes_key<S: Into<String>>(mut self, key: S) -> Self {
        self.aes_key = Some(key.into());
        self
    }

    /// Enable JSON output format
    pub fn with_json_format(mut self, json: bool) -> Self {
        self.json_format = json;
        self
    }
}

/// File entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// The path of the file within the pak/utoc
    pub path: AssetPath,
    /// The size of the file in bytes
    pub size: u64,
    /// Whether the file is compressed
    pub is_compressed: bool,
    /// The compression method used (if any)
    pub compression: Option<CompressionMethod>,
}

/// Unpacked file information
#[derive(Debug, Clone)]
pub struct UnpackedFile {
    /// The original path in the pak/utoc
    pub original_path: AssetPath,
    /// The path where the file was extracted to
    pub output_path: PathBuf,
    /// The size of the file in bytes
    pub size: u64,
    /// Any error that occurred during unpacking
    pub error: Option<String>,
}

/// Progress information for long operations
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Current progress (0-100)
    pub percentage: u8,
    /// Current operation description
    pub message: String,
    /// Number of items processed
    pub processed: u64,
    /// Total number of items to process
    pub total: u64,
}

/// Callback for progress updates
pub type ProgressCallback = Box<dyn FnMut(ProgressInfo) + Send + Sync>;

/// Information about a PAK file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PakFileInfo {
    /// Path to the PAK file
    pub file_path: PathBuf,
    /// Size of the PAK file in bytes
    pub file_size: u64,
    /// Number of files in the PAK
    pub file_count: usize,
    /// Total uncompressed size of all files
    pub total_uncompressed_size: u64,
    /// PAK file version
    pub version: String,
    /// Whether the file is encrypted
    pub is_encrypted: bool,
}

/// Configuration for the UE tools library
#[derive(Debug, Clone)]
pub struct UeToolsConfig {
    /// Default AES key to use if not specified in options
    pub default_aes_key: Option<String>,
    /// Whether to use parallel processing
    pub use_parallel: bool,
    /// Number of worker threads (0 = auto)
    pub worker_threads: usize,
}

impl Default for UeToolsConfig {
    fn default() -> Self {
        Self {
            default_aes_key: None,
            use_parallel: true,
            worker_threads: 0,
        }
    }
}

impl UeToolsConfig {
    /// Create new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default AES key
    pub fn with_default_aes_key<S: Into<String>>(mut self, key: S) -> Self {
        self.default_aes_key = Some(key.into());
        self
    }

    /// Enable or disable parallel processing
    pub fn with_parallel_processing(mut self, parallel: bool) -> Self {
        self.use_parallel = parallel;
        self
    }

    /// Set the number of worker threads
    pub fn with_worker_threads(mut self, threads: usize) -> Self {
        self.worker_threads = threads;
        self
    }

}