use serde::{Deserialize, Serialize};

use crate::extract::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Readability;

#[typetag::serde(name = "readability")]
impl Extract for Readability {
    fn extract(&self, body: &str) -> Readable {
        let (node, metadata) = readable_readability::Readability::new().parse(body);

        let mut content = Vec::<u8>::new();
        node.serialize(&mut content).unwrap();

        Readable {
            title: metadata.article_title,
            content: String::from_utf8(content).unwrap(),
            text: node.text_contents(),
        }
    }

    fn describe(&self) -> String {
        "Readability".to_string()
    }
}
