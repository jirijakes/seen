use isahc::http::Uri;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Video {
    pub transcript: String,
    pub title: String,
    #[serde(serialize_with = "http_serde::uri::serialize")]
    pub url: Uri,
}
