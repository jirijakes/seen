use std::collections::HashMap;

use isahc::http::{HeaderMap, Uri};
use isahc::prelude::*;
use isahc::{AsyncBody, Response};
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::document::*;
use crate::options::SeenOptions;
use crate::url_preferences::Preferences;

/// Content of a webpage .
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Page {
    /// HTTP headers with which the webpage was returned.
    #[serde(with = "http_serde::header_map")]
    pub headers: HeaderMap,
    /// Raw body of the webpage.
    pub body: String,
    /// URL from which the webpage was returned.
    #[serde(with = "http_serde::uri")]
    pub url: Uri,
}

#[derive(Debug, Diagnostic, Error)]
pub enum PageError {}

/// Turn response into a [`Page`] using given `extract`.
pub async fn make_page(mut res: Response<AsyncBody>) -> Result<Page, PageError> {
    let url = res.effective_uri().unwrap().clone();
    let headers = res.headers().clone();
    let body = res.text().await.unwrap();

    Ok(Page { headers, body, url })
}

impl Prepare for Page {
    fn prepare_document(
        &self,
        metadata: HashMap<String, Value>,
        options: &SeenOptions,
        preferences: Option<Preferences>,
    ) -> Document {
        let mut metadata = metadata;

        let extract = crate::options::extract(options, &preferences);

        let html =
            webpage::HTML::from_string(self.body.clone(), Some(self.url.to_string())).unwrap();
        let readable = extract.as_ref().extract(&self.body);

        let title = readable
            .title
            .as_ref()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                html.opengraph
                    .properties
                    .get("title")
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| html.title.as_ref().filter(|s| !s.is_empty()))
            .cloned();

        // TODO: More granular
        if let Some(host) = self.url.host() {
            metadata.insert("host".to_string(), serde_json::to_value(host).unwrap());
        }

        let md = futures::executor::block_on(crate::convert::md::html_to_md(&readable.content));

        Document {
            title: title.unwrap(),
            url: self.url.clone(),
            uuid: Uuid::new_v4(),
            content: Content::WebPage {
                text: readable.text,
                rich_text: Some(md),
            },
            metadata,
        }
    }
}
