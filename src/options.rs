use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::extract::Extraction;
use crate::url_preferences::Preferences;

/// Configuration options of Seen.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SeenOptions {
    /// Directory to store archived files.
    pub archive_dir: Option<PathBuf>,
    /// Show progress bar when in foreground.
    pub show_progress_bar: bool,
    /// Run in foreground.
    pub always_in_foreground: bool,
    /// Use Tor when downloading from the internet.
    pub use_tor: bool,
    /// Include timestamp in indexed documents.
    pub include_time: bool,
    /// Default extract.
    pub extract: Extraction,
}

pub fn extract<'a>(options: &'a SeenOptions, preferences: &'a Preferences) -> &'a Extraction {
    preferences.extract.as_ref().unwrap_or(&options.extract)
}
