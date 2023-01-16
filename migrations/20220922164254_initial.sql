CREATE TABLE IF NOT EXISTS "documents" (
	"id"	       INTEGER NOT NULL,
	"uuid"	       BLOB NOT NULL UNIQUE,
	"url"	       TEXT NOT NULL,
        "time"         TEXT NOT NULL,
	"title"        TEXT NOT NULL,
	"metadata"     TEXT NOT NULL DEFAULT '{}',
	"content_type" TEXT NOT NULL,
	PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE IF NOT EXISTS "url_preferences" (
	"pattern"	TEXT NOT NULL UNIQUE,
	"preferences"	TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS "webpage" (
       "plain"          TEXT NOT NULL,
       "rich"           TEXT NULL,
       "document"       INTEGER NOT NULL,
       FOREIGN KEY("document") REFERENCES "documents"("id")
);
