pub mod page;
pub mod video;

use std::collections::HashMap;

use isahc::http::Uri;
use mime::{Mime, HTML, IMAGE, PNG, TEXT, VIDEO};
pub use page::{make_page, Page, PageError};
use serde::Serialize;
use serde_json::Value;

use self::video::Video;
use crate::{document::*, options::SeenOptions, url_preferences::Preferences};

/// Pre-processed input to extraction algorithms.
///
/// Ideally the source would contain everything
#[derive(Clone, Debug, Serialize)]
pub enum Source {
    Page(Page),
    Video(Video),
}

impl Prepare for Source {
    fn prepare_document(
        &self,
        metadata: HashMap<String, Value>,
        options: &SeenOptions,
        preferences: Option<Preferences>,
    ) -> Document {
        match self {
            Source::Page(page) => page.prepare_document(metadata, options, preferences),
            Source::Video(_) => todo!(),
        }
    }
}
impl Source {
    pub fn url(&self) -> Option<&Uri> {
        match self {
            Source::Page(p) => Some(&p.url),
            Source::Video(v) => Some(&v.url), //v.url.as_ref(),
        }
    }
}

/// All supported types of documents.
pub enum SourceType {
    Image,
    Page,
    Video,
}

impl SourceType {
    /// Translate mime type (content type) into document type.
    /// If no such matching document type found, return `None`.
    pub fn from_mime(mime: &Mime) -> Option<SourceType> {
        match (mime.type_(), mime.subtype()) {
            (TEXT, HTML) => Some(SourceType::Page),
            (IMAGE, PNG) => Some(SourceType::Image),
            (VIDEO, _) => Some(SourceType::Video),
            _ => None,
        }
    }
}
