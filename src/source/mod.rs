pub mod page;
pub mod video;

use std::collections::HashMap;

use isahc::http::Uri;
use miette::Diagnostic;
use mime::{Mime, HTML, IMAGE, PNG, TEXT};
pub use page::{make_page, Page, PageError};
use serde::Serialize;
use serde_json::Value;

use crate::document::*;

#[derive(Clone, Debug, Serialize)]
pub enum Source {
    Page(Page),
}

impl Prepare for Source {
    fn prepare_document(&self, metadata: HashMap<String, Value>) -> Document {
        match self {
            Source::Page(page) => page.prepare_document(metadata),
        }
    }
}
impl Source {
    pub fn url(&self) -> Option<&Uri> {
        match self {
            Source::Page(p) => Some(&p.url),
        }
    }
}

/// All supported types of documents.
pub enum SourceType {
    Image,
    Page,
}

impl SourceType {
    /// Translate mime type (content type) into document type.
    /// If no such matching document type found, return `None`.
    pub fn from_mime(mime: &Mime) -> Option<SourceType> {
        match (mime.type_(), mime.subtype()) {
            (TEXT, HTML) => Some(SourceType::Page),
            (IMAGE, PNG) => Some(SourceType::Image),
            _ => None,
        }
    }
}
