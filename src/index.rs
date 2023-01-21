use std::cell::RefCell;
use std::fmt::Debug;
use std::path::Path;

use miette::Diagnostic;
use tantivy::collector::TopDocs;
use tantivy::directory::error::OpenDirectoryError;
use tantivy::directory::MmapDirectory;
use tantivy::query::{QueryParser, QueryParserError};
use tantivy::schema::{
    Cardinality, Field, Schema, TextFieldIndexing, TextOptions, INDEXED, STORED, TEXT,
};
use tantivy::{
    DateOptions, DatePrecision, DateTime, DocAddress, Document as TantivyDocument, Index,
    IndexReader, IndexWriter, Score, SnippetGenerator, TantivyError, Term,
};
use thiserror::Error;
use uuid::Uuid;

use crate::document::Document;

#[derive(Debug, Error, Diagnostic)]
pub enum IndexError {
    #[error("Could not open index directory.")]
    MmapDirectory(#[from] OpenDirectoryError),

    #[error("Index error.")]
    Tantivy(#[from] TantivyError),
}

#[derive(Debug, Error, Diagnostic)]
pub enum SearchError {
    #[error("Invalid query.")]
    Query(#[from] QueryParserError),

    #[error("Index error.")]
    Tantivy(#[from] TantivyError),
}

/// All tantivy fields that seen uses.
struct Fields {
    /// Title of the source (web page, video etc.).
    title: Field,
    /// Textual content of the source (article content, video speech etc.).
    content: Field,
    /// Time when the document was indexed.
    time: Field,
    /// Additional fields.
    meta: Field,
    /// UUID of the document.
    uuid: Field,
}

/// Holds all that is needed to maintain full-text index in memory,
/// so we don't have to create it every time.
pub struct SeenIndex {
    index: Index,
    query_parser: QueryParser,
    reader: IndexReader,
    writer: RefCell<IndexWriter>,
    fields: Fields,
}

impl SeenIndex {
    /// Create new seen index with the underlying tantivy index in directory 'path'.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<SeenIndex, IndexError> {
        let (schema, fields) = seen_schema();

        std::fs::create_dir_all(&path).unwrap();
        let dir = MmapDirectory::open(&path)?;
        let index = Index::open_or_create(dir, schema)?;
        let reader = index.reader()?;
        let writer = index.writer(100_000_000)?;

        let query_parser =
            QueryParser::for_index(&index, vec![fields.title, fields.content, fields.meta]);

        Ok(SeenIndex {
            index,
            query_parser,
            reader,
            fields,
            writer: RefCell::new(writer),
        })
    }

    /// Index a document. Returns tantivy docid.
    pub fn index(&self, document: &Document) -> Result<u64, IndexError> {
        let mut doc = TantivyDocument::new();

        let meta = serde_json::to_value(&document.metadata)
            .ok()
            .and_then(|j| j.as_object().cloned())
            .unwrap_or_else(serde_json::Map::new);

        doc.add_text(self.fields.title, &document.title);
        doc.add_text(self.fields.content, document.content.plain_text());
        doc.add_date(
            self.fields.time,
            DateTime::from_timestamp_secs(document.time.naive_utc().timestamp_millis()),
        );
        doc.add_bytes(self.fields.uuid, document.uuid.into_bytes());
        doc.add_json_object(self.fields.meta, meta);

        let mut writer = self.writer.borrow_mut();

        let id = writer.add_document(doc)?;

        writer.commit()?;

        Ok(id)
    }

    /// Search among documents using a tantivy query.
    pub fn search(&self, query: &str) -> Result<Vec<SearchHit>, SearchError> {
        let query = self.query_parser.parse_query(query)?;

        let searcher = self.reader.searcher();

        let top: Vec<(Score, DocAddress)> = searcher.search(&query, &TopDocs::with_limit(10))?;

        let snippet_generator = SnippetGenerator::create(&searcher, &query, self.fields.content)?;

        top.into_iter()
            .map(|(score, address)| {
                let doc = searcher.doc(address)?;
                let snippet = snippet_generator.snippet_from_doc(&doc);

                Ok(SearchHit {
                    score,
                    snippet: Self::highlight(snippet),
                    title: doc
                        .get_first(self.fields.title)
                        .and_then(|f| f.as_text())
                        .unwrap()
                        .to_string(),
                    uuid: doc
                        .get_first(self.fields.uuid)
                        .and_then(|f| uuid::Uuid::from_slice(f.as_bytes().unwrap()).ok())
                        .unwrap(),
                })
                // println!(
                //     "metadata: {:?}",
                //     doc.get_first(self.fields.meta).and_then(|f| f.as_json())
                // );
            })
            .collect()
    }

    /// Delete all documents with the given `uuid` from the index.
    pub fn delete(&self, uuid: &Uuid) -> Result<(), IndexError> {
        let mut writer = self.writer.borrow_mut();
        let term = Term::from_field_bytes(self.fields.uuid, &uuid.into_bytes());
        writer.delete_term(term);
        writer.commit()?;

        Ok(())
    }

    fn highlight(snippet: tantivy::Snippet) -> String {
        let mut result = String::new();
        let mut start_from = 0;

        for fragment_range in snippet.highlighted() {
            result.push_str(&snippet.fragment()[start_from..fragment_range.start]);
            result.push_str("***");
            result.push_str(&snippet.fragment()[fragment_range.clone()]);
            result.push_str("***");
            start_from = fragment_range.end;
        }

        result.push_str(&snippet.fragment()[start_from..]);
        result
    }
}

impl Debug for SeenIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeenIndex")
            .field("index", &self.index)
            .finish()
    }
}

/// Generates Tantivy [schema](tantivy::Schema) for Seen.
fn seen_schema() -> (Schema, Fields) {
    let mut schema_builder = Schema::builder();
    let text_field = TextFieldIndexing::default()
        .set_tokenizer("en_stem")
        .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default()
        .set_indexing_options(text_field)
        .set_stored();
    let time_options = DateOptions::from(INDEXED)
        .set_stored()
        .set_fast(Cardinality::MultiValues)
        .set_precision(DatePrecision::Seconds);
    let title = schema_builder.add_text_field("title", text_options.clone());
    let time = schema_builder.add_date_field("time", time_options);
    let content = schema_builder.add_text_field("content", text_options);
    let meta = schema_builder.add_json_field("meta", TEXT | STORED);
    let uuid = schema_builder.add_bytes_field("uuid", STORED);

    let schema = schema_builder.build();
    let fields = Fields {
        title,
        content,
        time,
        meta,
        uuid,
    };

    (schema, fields)
}

/// One search hit.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub score: Score,
    pub title: String,
    pub uuid: Uuid,
    pub snippet: String,
}
