use std::collections::HashMap;

use isahc::http::header::CONTENT_TYPE;
use isahc::http::Uri;
use isahc::Response;
use miette::Diagnostic;
use mime::Mime;
use serde_json::Value;
use thiserror::Error;

use crate::archive::archive_source;
use crate::document::{Content, Prepare};
use crate::extract::Extraction;
use crate::index::IndexError;
use crate::metadata::Metadata;
use crate::source::{make_page, Source, SourceType};
use crate::url_preferences::{self, Preferences, UrlPreferences};
use crate::Seen;

pub struct Job;

#[derive(Debug, Diagnostic, Error)]
pub enum JobError {
    #[error("HTTP error.")]
    HttpError(#[from] isahc::Error),
    #[error("Content type {0:?} not supported (yet?).")]
    MimeNotSupported(Mime),
    #[error("")]
    InvalidResponse,
    #[error("Adress was blacklisted")]
    Blacklisted,
    #[error("Index error.")]
    IndexError(#[from] IndexError),
}

pub async fn go(seen: &Seen, url: Uri, tags: &[String]) -> Result<(), JobError> {
    let default_metadata =
        HashMap::from([("tag".to_string(), serde_json::to_value(tags).unwrap())]);

    let source = download_source(seen, &url, &default_metadata).await?;

    index_source(seen, &url, source, default_metadata, tags).await
}

pub async fn download_source(
    seen: &Seen,
    url: &Uri,
    default_metadata: &HashMap<String, Value>,
) -> Result<Source, JobError> {
    // 1. Get preferences for URL (glob?)
    let url_preferences: Option<UrlPreferences> = url_preferences::for_url(&url, seen).await;

    let preferences = match url_preferences {
        Some(UrlPreferences::Blacklist) => Err(JobError::Blacklisted),
        Some(UrlPreferences::Preferences(s)) => Ok(Some(s)),
        None => Ok(None),
    }?;

    let response = seen.http_client.get_async(url).await?;

    // TODO: check status
    let _status = response.status();

    let ct = preferences.as_ref().and_then(|s| s.content_type.clone());
    let effective_ct = ct.unwrap_or(content_type(&response)?);
    let extract = crate::options::extract(&seen.options, &preferences);

    let source: Source = match SourceType::from_mime(&effective_ct) {
        Some(SourceType::Page) => make_page(response, extract)
            .await
            .map(Source::Page)
            .unwrap(),
        Some(SourceType::Image) => todo!(),
        None => Err(JobError::MimeNotSupported(effective_ct))?,
    };

    archive_source(seen, &source, default_metadata).await;

    Ok(source)
}

pub async fn index_source(
    seen: &Seen,
    url: &Uri,
    source: Source,
    default_metadata: HashMap<String, Value>,
    tags: &[String],
) -> Result<(), JobError> {
    let document = source.prepare_document(default_metadata);

    let _ = seen.index.index(&document)?;

    let url_s = url.to_string();

    let metadata = Metadata {
        tags: tags.to_vec(),
    };

    let document_id: i64 = {
        let mjs = serde_json::to_string(&metadata).unwrap();
        sqlx::query!(
            "INSERT INTO documents (uuid, url, title, metadata, content_type) VALUES (?, ?, ?, ?, ?)",
            document.uuid,
            url_s,
            document.title,
            mjs,
	    "webpage"
        )
        .execute(&seen.pool)
        .await
        .unwrap()
        .last_insert_rowid()
    };

    let q = match document.content {
        Content::WebPage { text, rich_text } => {
            sqlx::query!(
                "INSERT INTO webpage (plain, rich, document) VALUES (?, ?, ?)",
                text,
                rich_text,
                document_id
            )
            .execute(&seen.pool)
            .await
        }
    };

    println!("{q:?} — {:?}", document.uuid);

    Ok(())
}

/// Extract contenty type from given HTTP response.
fn content_type<B>(response: &Response<B>) -> Result<Mime, JobError> {
    let ct = response
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .map_err(|_| JobError::InvalidResponse)?;
    let mime: Mime = ct.parse().map_err(|_| JobError::InvalidResponse)?;
    Ok(mime)
}
