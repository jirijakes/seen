use std::collections::HashSet;

use isahc::http::Uri;
use mime::Mime;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{query_as, FromRow, Row};
use thiserror::Error;

use crate::extract::Extraction;
use crate::Seen;

#[derive(Debug, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    #[serde(with = "mime_serde")]
    pub content_type: Option<Mime>,
    pub extract: Option<Extraction>,

    // TODO Make more sophisticated: +add -remove
    pub tags: HashSet<String>,
    // tor
    // archive
    // include time
}

/// Find preferences for `url`.
pub async fn for_url(url: &Uri, seen: &Seen) -> Option<UrlPreferences> {
    query_as("SELECT preferences FROM url_preferences WHERE ? GLOB pattern")
        .bind(url.to_string())
        .fetch_optional(&seen.pool)
        .await
        .ok()
        .flatten()
}

#[derive(Debug, Error)]
#[error("Url preferences in database is in invalid format.")]
struct InvalidPreferencesFormat;

/// URL-specific preferences. These preferences are stored in database
/// and allow users to override any default preferences.
#[derive(Debug)]
pub enum UrlPreferences {
    Blacklist,
    Preferences(Preferences),
}

impl FromRow<'_, SqliteRow> for UrlPreferences {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let s: &str = row.try_get("preferences")?;

        if s == "blacklist" {
            Ok(UrlPreferences::Blacklist)
        } else if let Ok(s) = serde_json::from_str::<Preferences>(s) {
            Ok(UrlPreferences::Preferences(s))
        } else {
            Err(sqlx::Error::ColumnDecode {
                index: "preferences".to_string(),
                source: Box::new(InvalidPreferencesFormat),
            })
        }
    }
}

mod mime_serde {

    use mime::Mime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(mime: &Option<Mime>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match mime {
            Some(m) => s.serialize_str(m.as_ref()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<Mime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(d)?;

        match s {
            Some(m) => Ok(Some(m.parse().map_err(serde::de::Error::custom)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Preferences;

    #[test]
    fn deserialize_preferences() {
        let j = r#" {  } "#;
        let s = serde_json::from_str::<Preferences>(j);
        println!("{s:?}");
    }
}
