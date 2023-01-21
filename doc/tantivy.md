# Tantivy index

*Note: For more user-oriented view see [Search queries](queries.md).*

For indexing text, *Seen* uses [Tantivy](https://docs.rs/tantivy/), which is a low-level search engine library written
in Rust. This document describes how the index is used. The whole schema, including fields, is defined in
[src/index.rs](/file?name=src/index.rs&ci=tip) in the function `seen_schema()`.

## Fields

#### `title`

Title of the document. It is usually extracted from the source, for example from web page title.

#### `content`

Textual content of the document. The origin of the content depends on type of source. In case of web pages,
it is the body of the article. Content of videos is transcription of its audio track.

#### `time`

Time when the document was added to index. It is of type `date`.

#### `uuid`

UUID of the document used to correlate Tantivy documents with the document in database. It is of type `bytes`.

#### `meta`

Document's additional metadata. The field is of type `json`. Currently, the metadata have following content:

``` json
{
  "tag": ["…", "…"],
  "host": "…",
  "indextime": {
    "daypart": "…",
    "weekday": "…",
    "season": "…",
    "month": "…"
  }
}
```

Valid values:

- `daypart`: morning, noon, afternoon, evening, night
- `weekday`: monday, tuesday, wednesday, thursday, friday, saturday, sunday
- `season`: summer, autumn, winter, spring (at the moment naïve and inaccurate approach, assuming northern hemisphere only)
- `month`: january, february, march, april, may, june, july, august, september, october, november, december
