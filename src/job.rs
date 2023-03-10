use std::collections::HashMap;

use chrono::{DateTime, Local};
use indicatif::*;
use isahc::http::header::CONTENT_TYPE;
use isahc::http::Uri;
use isahc::{Metrics, Response, ResponseExt};
use miette::Diagnostic;
use mime::Mime;
use serde_json::Value;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration};
use uuid::Uuid;

use crate::archive::archive_source;
use crate::document::{Content, Prepare};
use crate::index::IndexError;
use crate::metadata::Metadata;
use crate::source::{make_page, Source, SourceType};
use crate::url_preferences::{self, Preferences, UrlPreferences};
use crate::{Seen, SeenError};

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

    #[error("Database error.")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Seen error.")]
    SeenError(#[from] SeenError),
}

pub async fn go(
    seen: &Seen,
    url: Uri,
    tags: &[String],
    archive: bool,
    dry_run: bool,
) -> Result<(), JobError> {
    let default_metadata =
        HashMap::from([("tag".to_string(), serde_json::to_value(tags).unwrap())]);

    // 1. Get preferences for URL (glob?)
    let url_preferences: Option<UrlPreferences> = url_preferences::for_url(&url, seen).await;

    let preferences: Preferences = match url_preferences {
        Some(UrlPreferences::Blacklist) => Err(JobError::Blacklisted),
        Some(UrlPreferences::Preferences(s)) => Ok(s),
        None => Ok(Default::default()),
    }?;

    let multi = MultiProgress::new();
    let sty = ProgressStyle::with_template("{bar:40.green/yellow} {pos:>7}/{len:7}").unwrap();

    let total_pb = multi.add(ProgressBar::new(3));
    total_pb.set_style(sty.clone());
    total_pb.tick();

    let download_pb = multi.add(ProgressBar::new(0));
    download_pb.set_style(sty.clone());

    let time = Local::now();

    total_pb.inc(1);
    let source = download_source(seen, &url, &preferences, download_pb.clone()).await?;
    multi
        .println(format!(
            "Source download finished in {:?}.",
            download_pb.elapsed()
        ))
        .unwrap();
    total_pb.finish_and_clear();

    if archive && !dry_run {
        let archive_pb = multi.add(ProgressBar::new(100));
        archive_pb.set_style(sty.clone());
        archive_pb.set_position(0);

        archive_source(seen, &source, &default_metadata, time).await;

        archive_pb.set_position(100);
        archive_pb.finish_and_clear();
        multi.println("Archived.").unwrap();
    }

    let res = if !dry_run {
        let index_pb = multi.add(ProgressBar::new(100));
        index_pb.set_style(sty);
        index_pb.set_position(0);

        // Index the source.
        let res = index_source(
            seen,
            &url,
            source,
            &preferences,
            default_metadata,
            time,
            tags,
        )
        .await;

        index_pb.set_position(100);
        index_pb.finish_and_clear();
        multi.println("Indexed.").unwrap();

        res
    } else {
        Ok(())
    };

    multi.println("Done.").unwrap();
    multi.clear().unwrap();

    res
}

/// Regularly checks given download metrics and updates progress bar accordingly.
/// It is expected to be cancelled externally once the download has finished,
/// e. g. by [`tokio::select!`].
async fn download_progress(m: Metrics, pb: ProgressBar) {
    let mut int = interval(Duration::from_millis(10));

    loop {
        int.tick().await;
        let (pos, tot) = m.download_progress();
        pb.set_length(tot);
        pb.set_position(pos);
    }
}

pub async fn download_source(
    seen: &Seen,
    url: &Uri,
    preferences: &Preferences,
    progress_bar: ProgressBar,
) -> Result<Source, JobError> {
    let response = seen.http_client.get_async(url).await?;

    // TODO: check status
    let _status = response.status();

    let ct = preferences.content_type.clone();
    let effective_ct = ct.unwrap_or(content_type(&response)?);

    let (downloaded_signal, downloaded) = oneshot::channel::<()>();

    if let Some(m) = response.metrics().cloned() {
        tokio::spawn(async move {
            tokio::select! {
                _ = downloaded => {
                    progress_bar.finish_and_clear();
                }
                _ = download_progress(m.clone(), progress_bar.clone()) => { }
            }
        });
    }

    let source: Source = match SourceType::from_mime(&effective_ct) {
        Some(SourceType::Page) => make_page(response, downloaded_signal)
            .await
            .map(Source::Page)
            .unwrap(),
        Some(SourceType::Image) => todo!(),
        Some(SourceType::Video) => todo!(),
        None => Err(JobError::MimeNotSupported(effective_ct))?,
    };

    Ok(source)
}

pub async fn index_source(
    seen: &Seen,
    url: &Uri,
    source: Source,
    preferences: &Preferences,
    default_metadata: HashMap<String, Value>,
    time: DateTime<Local>,
    tags: &[String],
) -> Result<(), JobError> {
    // We do not want to index the same URL if it already exists.
    // Therefore, let's first delete documents bound to this URL if they
    // already exist
    delete_existing(seen, url).await?;

    let document = source.prepare_document(default_metadata, &seen.options, preferences, time);

    let _ = seen.index.index(&document)?;

    let url_s = url.to_string();

    let metadata = Metadata {
        tags: tags.to_vec(),
    };

    let document_id: i64 = {
        let mjs = serde_json::to_string(&metadata).unwrap();
        sqlx::query!(
            "INSERT INTO documents (uuid, url, title, time, metadata, content_type) VALUES (?, ?, ?, ?, ?, ?)",
            document.uuid,
            url_s,
            document.title,
            document.time,
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

    println!("{q:?} ??? {:?}", document.uuid);

    Ok(())
}

/// Delete documents coming from `url` if they exist. If no document exists,
/// nothing happens.
async fn delete_existing(seen: &Seen, url: &Uri) -> Result<(), JobError> {
    let url_s = url.to_string();

    #[rustfmt::skip]
    let existing: Option<Uuid> =
        sqlx::query!(
            r#"SELECT uuid AS "uuid: Uuid" FROM documents WHERE url = ?"#,
            url_s
        )
        .fetch_optional(&seen.pool)
        .await?
        .map(|r| r.uuid);

    // If a document with the same URL already exists, we are updating it.
    // Updating with tantivy equals to deleting + inserting again newly.
    if let Some(uuid) = existing {
        Ok(seen.delete(&uuid).await?)
    } else {
        Ok(())
    }
}

/// Extract content type from given HTTP response.
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
