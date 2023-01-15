use std::collections::HashMap;
use std::path::Path;

use futures::StreamExt;
use isahc::http::{HeaderMap, Uri};
use miette::Diagnostic;
use serde::Deserialize;
use serde_json::{json, Value};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::OffsetDateTime;
use tokio::fs::{read_dir, read_to_string, File};
use tokio::io::AsyncWriteExt;
// use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;

use crate::source::page::recover_page;
use crate::source::Source;
use crate::url_preferences::UrlPreferences;
use crate::Seen;

#[derive(Debug, Error, Diagnostic)]
pub enum RecoverError {
    #[error("Could not read archive file.")]
    File(#[from] std::io::Error),

    #[error("Could not decode archive file.")]
    Decode(#[from] serde_json::Error),
}

/// Store a copy of the source information for archival purposes.
/// Such a store can be later re-indexed by TODO.
pub async fn archive_source(seen: &Seen, source: &Source, metadata: &HashMap<String, Value>) {
    let time = OffsetDateTime::now_local()
        .unwrap_or(OffsetDateTime::now_utc())
        .format(&Rfc3339)
        .ok();

    let json = json!({
         "metadata": metadata,
         "source": source,
         "time": time
    });

    let filename = time::OffsetDateTime::now_utc()
        .format(&format_description!(
            "[year][month][day][hour][minute][second][subsecond]"
        ))
        .unwrap()
        .to_string();

    let mut file = File::create(seen.archive_dir().join(format!("{filename}.json")))
        .await
        .unwrap();
    let mut buf = vec![];

    serde_json::to_writer_pretty(&mut buf, &json).unwrap();
    file.write_all(&buf).await.unwrap();
}

// TODO: Error handling
pub async fn recover(seen: &Seen) -> Result<(), RecoverError> {
    let rd = read_dir(seen.archive_dir()).await.unwrap();

    let _ = ReadDirStream::new(rd)
        .filter_map(|d| async { d.ok() })
        .filter_map(|d| async move {
            if let Ok(file_type) = d.file_type().await {
                if file_type.is_file() {
                    Some(d.path())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .then(|s| recover_source(seen, s))
        .collect::<Vec<_>>()
        .await;

    Ok(())
}

pub async fn recover_source(seen: &Seen, file: impl AsRef<Path>) -> Result<(), RecoverError> {
    let archived = serde_json::from_str::<Archived>(&read_to_string(file).await?)?;

    match archived.source {
        ArchivedSource::Page(p) => {
            let url_preferences: Option<UrlPreferences> =
                crate::url_preferences::for_url(&p.url, seen).await;

            let preferences = match url_preferences {
                Some(UrlPreferences::Blacklist) => Err(todo!()),
                Some(UrlPreferences::Preferences(s)) => Ok(Some(s)),
                None => Ok(None),
            }
            .unwrap();

            if let Ok(page) = recover_page(p, crate::options::extract(&seen.options, &preferences))
            {
                crate::job::index_source(
                    seen,
                    &page.url.clone(),
                    Source::Page(page),
                    archived.metadata,
                    &[],
                )
                .await
                .unwrap()
            }
        }
    }

    Ok(())
}

/// Archived source with all the available metadata. The metadata includes
/// user input.
#[derive(Debug, Clone, Deserialize)]
pub struct Archived {
    pub metadata: HashMap<String, Value>,
    pub source: ArchivedSource,
}

/// Archived source.
#[derive(Debug, Clone, Deserialize)]
pub enum ArchivedSource {
    Page(ArchivedPage),
}

/// Archived page.
#[derive(Debug, Clone, Deserialize)]
pub struct ArchivedPage {
    #[serde(deserialize_with = "http_serde::header_map::deserialize")]
    pub headers: HeaderMap,
    pub body: String,
    #[serde(deserialize_with = "http_serde::uri::deserialize")]
    pub url: Uri,
}
