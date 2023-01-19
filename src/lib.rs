mod convert;
pub mod document;
//mod download;
pub mod archive;
mod export;
mod extract;
mod index;
pub mod job;
mod metadata;
mod options;
mod readability;
mod source;
mod url_preferences;

use std::path::PathBuf;
use std::rc::Rc;

use directories::ProjectDirs;
use futures::{StreamExt, TryStreamExt};
use index::IndexError;
use isahc::prelude::Configurable;
use isahc::HttpClient;
use miette::{Diagnostic, Result};
use options::SeenOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::fs::read_to_string;
use uuid::Uuid;

use crate::document::{Content, Document};
use crate::index::SeenIndex;

#[derive(Debug)]
pub struct Seen {
    /// HTTP client.
    http_client: HttpClient,
    /// Sqlite pol.
    pool: SqlitePool,
    /// Full-text index.
    index: Rc<SeenIndex>,
    /// Project directories.
    dirs: ProjectDirs,
    /// Configuration options of the seesion.
    options: SeenOptions,
}

#[derive(Debug, Error, Diagnostic)]
pub enum SeenError {
    #[error("Could not create HTTP client.")]
    HttpClient(#[from] isahc::Error),

    #[error("Could not open sqlite connection.")]
    Sql(#[from] sqlx::Error),

    #[error("Could not open seen index.")]
    Index(#[from] IndexError),

    #[error("Could not load configuration file: {0}")]
    Options(String),
}

impl Seen {
    /// Create new seen session with its database and full-text index.
    pub async fn new(_config: &Option<PathBuf>) -> Result<Seen, SeenError> {
        let dirs = ProjectDirs::from("com.jirijakes", "", "Seen").ok_or_else(|| {
            SeenError::Options("Could not load directory for configuration files".to_string())
        })?;

        let options = match read_to_string(dirs.config_dir().join("config.toml")).await {
            Ok(s) => toml::from_str(&s).map_err(|e| SeenError::Options(e.to_string())),
            Err(_) => Ok(Default::default()),
        }?;

        let http_client = HttpClient::builder()
            .metrics(true)
            .max_download_speed(1024)
            .build()?;
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(dirs.data_dir().join("seen.db"))
                    .create_if_missing(true),
            )
            .await?;

        sqlx::migrate!().run(&pool).await.unwrap();

        let index = Rc::new(SeenIndex::new(dirs.data_dir().join("index"))?);

        Ok(Seen {
            http_client,
            pool,
            index,
            dirs,
            options,
        })
    }

    /// Search among documents using a tantivy query.
    pub fn search(&self, query: &str) -> Result<Vec<index::SearchHit>, index::SearchError> {
        self.index.search(query)
    }

    /// Obtain content for given `partial_document` and return all as one [`Document`].
    async fn fill_content(&self, partial_document: PartialDocument) -> Result<Document, SeenError> {
        match partial_document.content_type {
            ContentType::WebPage => {
                let c = sqlx::query!(
                    r#"
SELECT *
FROM webpage
LEFT JOIN documents ON webpage.document = documents.id
WHERE documents.uuid = ?"#,
                    partial_document.uuid
                )
                .fetch_one(&self.pool)
                .await?;

                Ok(Document {
                    title: partial_document.title,
                    url: partial_document.url.parse().unwrap(),
                    uuid: partial_document.uuid,
                    time: partial_document.time,
                    content: Content::WebPage {
                        text: c.plain,
                        rich_text: c.rich,
                    },
                    metadata: serde_json::from_str(&partial_document.metadata).unwrap(),
                })
            }
        }
    }

    /// List all indexed documents.
    pub async fn list(&self) -> Result<Vec<Document>, SeenError> {
        sqlx::query_as_unchecked!(
            PartialDocument,
            "SELECT id, uuid, url, time, title, content_type, metadata FROM documents",
        )
        .fetch(&self.pool)
        .map_err(|e| e.into())
        .and_then(|d| self.fill_content(d))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect()
    }

    /// Obtain a complete document by its unique identifier.
    pub async fn get(&self, uuid: &Uuid) -> Result<Document, SeenError> {
        let document = sqlx::query_as_unchecked!(
            PartialDocument,
            "SELECT id, uuid, url, time, title, content_type, metadata FROM documents WHERE uuid = ?",
            uuid
        )
        .fetch_one(&self.pool)
        .await?;
        self.fill_content(document).await
    }

    /// Returns directory, which stores seen archive.
    pub fn archive_dir(&self) -> PathBuf {
        self.options
            .archive_dir
            .clone()
            .unwrap_or_else(|| self.dirs.data_dir().join("archive"))
    }
}

#[derive(sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
enum ContentType {
    WebPage,
}

/// Document without fully fetched content. Use `fill_content` to obtain
/// complete document.
#[allow(unused)]
struct PartialDocument {
    id: i64,
    title: String,
    uuid: Uuid,
    url: String,
    time: OffsetDateTime,
    content_type: ContentType,
    metadata: String,
}
