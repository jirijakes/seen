use std::collections::HashMap;

use isahc::http::{HeaderMap, Uri};
use isahc::prelude::*;
use isahc::{AsyncBody, Response};
use miette::Diagnostic;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::archive::ArchivedPage;
use crate::document::*;
use crate::extract::*;

/// Content of a webpage .
#[derive(Clone, Debug, Serialize)]
pub struct Page {
    /// HTTP headers with which the webpage was returned.
    #[serde(serialize_with = "http_serde::header_map::serialize")]
    pub headers: HeaderMap,
    /// Raw body of the webpage.
    pub body: String,
    /// URL from which the webpage was returned.
    #[serde(serialize_with = "http_serde::uri::serialize")]
    pub url: Uri,
    // Extracte information from the website's content.
    // #[serde(skip)]
    // pub html: webpage::HTML,
    // Readable content of the webpage.
    // #[serde(skip)]
    // pub readable: Readable,
}

#[derive(Debug, Diagnostic, Error)]
pub enum PageError {}

/// Turn response into a [`Page`] using given `extract`.
pub async fn make_page(
    mut res: Response<AsyncBody>,
    extract: &Extraction,
) -> Result<Page, PageError> {
    let url = res.effective_uri().unwrap().clone();
    let headers = res.headers().clone();
    let body = res.text().await.unwrap();

    make_page_internal(url, headers, body, extract)
}

pub fn recover_page(page: ArchivedPage, extract: &Extraction) -> Result<Page, PageError> {
    make_page_internal(page.url, page.headers, page.body, extract)
}

fn make_page_internal(
    url: Uri,
    headers: HeaderMap,
    body: String,
    extract: &Extraction,
) -> Result<Page, PageError> {
    let html = webpage::HTML::from_string(body.clone(), Some(url.to_string())).unwrap();
    let readable = extract.as_ref().extract(&body);

    Ok(Page {
        headers,
        body,
        url,
        html,
        readable,
    })
}

impl Page {
    /// From this downloaded page, obtain its most likely title.
    pub fn title(&self) -> Option<String> {
        self.readable
            .title
            .as_ref()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                self.html
                    .opengraph
                    .properties
                    .get("title")
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| self.html.title.as_ref().filter(|s| !s.is_empty()))
            .cloned()
    }
}

impl Prepare for Page {
    fn prepare_document(&self, metadata: HashMap<String, Value>) -> Document {
        let mut metadata = metadata;

        // TODO: More granular
        if let Some(host) = self.url.host() {
            metadata.insert("host".to_string(), serde_json::to_value(host).unwrap());
        }

        let md =
            futures::executor::block_on(crate::convert::md::html_to_md(&self.readable.content));

        Document {
            title: self.title().unwrap(),
            url: self.url.clone(),
            uuid: Uuid::new_v4(),
            content: Content::WebPage {
                text: self.readable.text.clone(),
                rich_text: Some(md),
            },
            metadata,
        }
    }
}
