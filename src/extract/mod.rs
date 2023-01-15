use core::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// Common trait of objects that know how to extract content and metadata from
/// webpages. Each object can serialize and deserialize its own settings in
/// order to keep them in database.
#[typetag::serde(tag = "extractor")]
pub trait Extract: Send {
    /// Extract content and metadata from given HTML page.
    fn extract(&self, body: &str) -> Readable;

    /// Give us textual representation of itself, including interesting settings.
    fn describe(&self) -> String;
}

/// Human-readable extract from a webpage.
#[derive(Clone, Debug)]
pub struct Readable {
    pub title: Option<String>,
    // pub byline: Option<String>,
    pub content: String,
    pub text: String,
    // pub excerpt: Option<String>,
}

/// Newtype to give material shape to `Box<dyn Extract> so we can attach
/// `Debug` instance to it. There is no other purpose.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Extraction(Box<dyn Extract>);

impl Deref for Extraction {
    type Target = Box<dyn Extract>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for Extraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

impl Default for Extraction {
    fn default() -> Self {
        Extraction(Box::new(crate::readability::Readability))
    }
}
