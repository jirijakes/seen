// use reqwest::blocking::{Request, Response};
// use reqwest::header::{ToStrError, CONTENT_TYPE};
// use reqwest::Method;

use miette::Diagnostic;
use mime::Mime;
use thiserror::Error;

use crate::source::*;
use crate::{readability, Seen};

/*
#[derive(Debug, Diagnostic, Error)]
pub enum DownloadError {
    #[error("Could not download {url}")]
    NetworkError { url: Url, source: reqwest::Error },
    #[error("Server did not return status code of success")]
    HttpError(#[source] reqwest::Error),
    #[error("Server returned an invalid content type: {0}")]
    InvalidContentType(String),
    #[error("Server returned an invalid header")]
    InvalidHeader(#[from] ToStrError),
    #[error("Error arised when making a page document from HTTP response")]
    PageError(#[from] PageError),
}

// TODO: Probably need to use isahc or hyper which allows finer control over headers/body.
pub fn download(seen: &Seen, url: Url) -> Result<Document, DownloadError> {
    // TODO: Some HEAD requests do not return content-type
    let head = call(seen, Request::new(Method::HEAD, url.clone()))?;

    let document_type = match head.error_for_status() {
        Ok(res) => Ok(DocumentType::from_mime(&content_type(&res)?)),
        Err(e) => Err(DownloadError::HttpError(e)),
    }?;

    if document_type.is_none() {
        todo!("Unknown mime type, how to report?")
    } else {
        let response = call(seen, Request::new(Method::GET, url))?;

        match response.error_for_status() {
            Ok(res) => match DocumentType::from_mime(&content_type(&res)?) {
                Some(DocumentType::Page) => {
                    Ok(make_page(res, Box::new(readability::Readability)).map(Document::Page)?)
                }
                Some(DocumentType::Image) => todo!("Indexing images not yet implemented"),
                None => todo!("Unknown mime type, how to report?"),
            },
            Err(e) => Err(DownloadError::HttpError(e)),
        }
    }
}

fn call(seen: &Seen, request: Request) -> Result<Response, DownloadError> {
    let url = request.url().clone();
    seen.http_client
        .execute(request)
        .map_err(|e| DownloadError::NetworkError { url, source: e })
}

/// Extract contenty type from given HTTP response.
fn content_type(res: &Response) -> Result<Mime, DownloadError> {
    let ct = res.headers()[CONTENT_TYPE].to_str()?;
    let mime: Mime = ct
        .parse()
        .map_err(|_| DownloadError::InvalidContentType(ct.to_string()))?;
    Ok(mime)
}
*/
