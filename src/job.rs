use std::collections::HashMap;

use indicatif::*;
use isahc::http::header::CONTENT_TYPE;
use isahc::http::Uri;
use isahc::{Metrics, Response, ResponseExt};
use miette::Diagnostic;
use mime::Mime;
use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration};

use crate::archive::archive_source;
use crate::document::{Content, Prepare};
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

    let preferences = match url_preferences {
        Some(UrlPreferences::Blacklist) => Err(JobError::Blacklisted),
        Some(UrlPreferences::Preferences(s)) => Ok(Some(s)),
        None => Ok(None),
    }?;

    let multi = MultiProgress::new();
    let sty = ProgressStyle::with_template("{bar:40.green/yellow} {pos:>7}/{len:7}").unwrap();

    let total_pb = multi.add(ProgressBar::new(5));
    total_pb.set_style(sty.clone());
    total_pb.tick();

    let download_pb = multi.add(ProgressBar::new(0));
    download_pb.set_style(sty.clone());

    let time = OffsetDateTime::now_utc();

    total_pb.inc(1);
    let source = download_source(seen, &url, preferences.as_ref(), download_pb.clone()).await?;
    multi
        .println(format!(
            "Source download finished in {:?}.",
            download_pb.elapsed()
        ))
        .unwrap();

    if archive && !dry_run {
        archive_source(seen, &source, &default_metadata, time).await;
    }

    let res = if !dry_run {
        index_source(
            seen,
            &url,
            source,
            preferences,
            default_metadata,
            time,
            tags,
        )
        .await
    } else {
        Ok(())
    };

    total_pb.finish_and_clear();
    multi.println("Done.").unwrap();
    multi.clear().unwrap();

    res
}

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
    preferences: Option<&Preferences>,
    progress_bar: ProgressBar,
) -> Result<Source, JobError> {
    let response = seen.http_client.get_async(url).await?;

    // TODO: check status
    let _status = response.status();

    let ct = preferences.as_ref().and_then(|s| s.content_type.clone());
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
    preferences: Option<Preferences>,
    default_metadata: HashMap<String, Value>,
    time: OffsetDateTime,
    tags: &[String],
) -> Result<(), JobError> {
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

    println!("{q:?} — {:?}", document.uuid);

    Ok(())
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
