use isahc::http::Uri;

pub struct Video {
    pub transcript: String,
    pub title: String,
    pub url: Option<Uri>,
}
