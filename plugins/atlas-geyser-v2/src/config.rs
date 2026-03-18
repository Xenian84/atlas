use serde_derive::{Deserialize, Serialize};

/// Configuration loaded from the JSON file passed to --geyser-plugin-config.
///
/// Minimal example:
/// ```json
/// {
///   "libpath": "/path/to/libatlas_geyser.so",
///   "connection_str": "host=localhost user=atlas dbname=atlas password=secret"
/// }
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtlasGeyserConfig {
    /// Full PostgreSQL connection string.
    /// e.g. "host=127.0.0.1 port=5432 user=atlas dbname=atlas password=secret"
    pub connection_str: String,

    /// Number of parallel writer threads / DB connections.
    /// Each thread holds one long-lived postgres connection.
    /// Default: 4
    #[serde(default = "default_threads")]
    pub threads: usize,

    /// Maximum number of account updates to buffer before dropping.
    /// If the buffer is full the update is logged and discarded — the
    /// validator thread is NEVER blocked.
    /// Default: 500_000
    #[serde(default = "default_channel_capacity")]
    pub channel_capacity: usize,

    /// How many accounts to accumulate before flushing to postgres (per worker).
    /// Default: 250
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Log a progress line every N accounts written.
    /// Default: 100_000
    #[serde(default = "default_log_every")]
    pub log_every: u64,
}

fn default_threads() -> usize { 4 }
fn default_channel_capacity() -> usize { 500_000 }
fn default_batch_size() -> usize { 250 }
fn default_log_every() -> u64 { 100_000 }
