use std::collections::HashMap;

use chrono::{DateTime, Local};
use isahc::http::Uri;
use serde_json::Value;
use uuid::Uuid;

use crate::options::SeenOptions;
use crate::url_preferences::Preferences;

/// Marks types that can be turned into [`documents`](Document).
pub trait Prepare {
    /// Generate [`Document`] from an object and additional data.
    fn prepare_document(
        &self,
        metadata: HashMap<String, Value>,
        options: &SeenOptions,
        preferences: &Preferences,
        time: DateTime<Local>,
    ) -> Document;
}

/// Document describes object that is fully prepared to be stored and indexed.
#[derive(Clone, Debug)]
pub struct Document {
    /// Title of the document, typically extracted from the source.
    pub title: String,

    /// Unique identifier of the document.
    pub uuid: Uuid,

    /// Time of indexing.
    pub time: DateTime<Local>,

    /// Original URL.
    pub url: Uri,

    /// Textual content of the document.
    pub content: Content,

    /// Other optional fields.
    pub metadata: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub enum Content {
    WebPage {
        /// Main content of the web page in plain text.
        text: String,

        /// Formatted text in Markdown for displaying. If none
        /// is present, `text` will be used.
        rich_text: Option<String>,
    },
}

impl Content {
    pub fn plain_text(&self) -> &str {
        match self {
            Content::WebPage { text, .. } => text,
        }
    }
}
