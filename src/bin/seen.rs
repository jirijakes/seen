use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use comfy_table::{presets, Attribute, Cell, CellAlignment, Table};
use futures::StreamExt;
use isahc::http::Uri;
use miette::Result;
use seen::document::Content;
use seen::Seen;
use sqlx::migrate::Migrator;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let seen = Seen::new(&args.config).await?;

    let migrator = Migrator::new(Path::new("migrations")).await.unwrap();
    migrator.run(&seen.pool).await.unwrap();

    match args.command {
        Command::Add(Add { url, tags }) => {
            seen::job::go(&seen, url, &tags).await?;
        }
        Command::Get(Get { uuid: id }) => {
            if let Ok(doc) = seen.get(&id).await {
                println!("{}", doc.title);
                println!();
                println!("{}", doc.url);
                println!();
                println!("{:?}", doc.metadata);
                println!();
                match doc.content {
                    Content::WebPage { text, rich_text } => {
                        // println!("{}", textwrap::fill(&text, 70));
                        println!("{}", text);
                        println!("\n\n-----------\n\n");
                        termimad::print_text(&rich_text.unwrap());
                    }
                };
                // println!("{}", doc.content);
            } else {
                println!("Nada.");
            }
        }
        Command::Search(Search { query }) => {
            futures::stream::iter(seen.index.search(&query)?)
                .filter_map(|hit| async { seen.get(&hit.uuid).await.map(|doc| (hit, doc)).ok() })
                .for_each(|(hit, document)| async move {
                    let mut table = Table::new();

                    table.load_preset(presets::NOTHING);

                    table.add_row(vec![
                        Cell::new("Title")
                            .add_attribute(Attribute::Bold)
                            .set_alignment(CellAlignment::Right),
                        Cell::new(&hit.title),
                    ]);

                    table.add_row(vec![
                        Cell::new("URL")
                            .add_attribute(Attribute::Bold)
                            .set_alignment(CellAlignment::Right),
                        Cell::new(&document.url),
                    ]);

                    if let Some(tags) = document.metadata.get("tags") {
                        if let Ok(tags) = serde_json::from_value::<Vec<String>>(tags.clone()) {
                            table.add_row(vec![
                                Cell::new("Tags")
                                    .add_attribute(Attribute::Bold)
                                    .set_alignment(CellAlignment::Right),
                                Cell::new(tags.join(", ")),
                            ]);
                        }
                    }

                    // table.add_row(vec![
                    //     Cell::new("UUID")
                    //         .add_attribute(Attribute::Bold)
                    //         .set_alignment(CellAlignment::Right),
                    //     Cell::new(&hit.uuid.to_string()),
                    // ]);

                    // table.add_row(vec![
                    //     Cell::new("Score")
                    //         .add_attribute(Attribute::Bold)
                    //         .set_alignment(CellAlignment::Right),
                    //     Cell::new(&hit.score.to_string()),
                    // ]);

                    table.add_row(vec![
                        Cell::new("Snippet")
                            .add_attribute(Attribute::Bold)
                            .set_alignment(CellAlignment::Right),
                        Cell::new(termimad::term_text(&hit.snippet)),
                    ]);

                    println!("{table}\n");
                })
                .await;
        }
        Command::List => {
            let mut table = Table::new();

            table.load_preset(presets::NOTHING);

            seen.list().await?.into_iter().for_each(|d| {
                let t = match d.content {
                    Content::WebPage { .. } => "webpage",
                };
                table.add_row(vec![d.uuid.to_string(), t.to_string(), d.title]);
            });

            println!("{table}");
        }
        Command::Recover(_) => seen::archive::recover(&seen).await?,
        Command::Settings(_) => {}
    }

    Ok(())
}

#[derive(Parser, Debug)]
struct Args {
    /// Custom location of configuration file.
    #[arg(short, long, id = "FILE")]
    config: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
struct Get {
    /// Obtain document by UUID
    uuid: Uuid,
}

#[derive(Parser, Debug)]
struct Search {
    /// Search content using a query.
    query: String,
}

#[derive(Parser, Debug)]
struct Add {
    /// URL to remember
    url: Uri,

    /// Add tag (can be used repeatedly)
    #[arg(short, long = "tag", id = "TAG")]
    tags: Vec<String>,
}

#[derive(Parser, Debug)]
struct Recover {
    /// Directory with archive files.
    dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Settings {
    /// Show existing settings.
    Get(GetSettings),
    /// Add or change settings.
    Set(SetSettings),
}

#[derive(Parser, Debug)]
pub struct GetSettings {
    /// URL to get the settings for.
    url: Option<Uri>,
    /// Provided argument is a glob pattern.
    #[arg(long)]
    glob: bool,
    /// Get global settings instead of per URL.
    #[arg(long)]
    global: bool,
}

#[derive(Parser, Debug)]
pub struct SetSettings {
    /// URL to get the settings for.
    pattern: Option<String>,
    /// Set global settings instead of per URL.
    #[arg(long)]
    global: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Add new URL.
    Add(Add),
    /// Search among documents.
    Search(Search),
    /// Obtain document directly.
    Get(Get),
    /// List indexed documents.
    List,
    /// Recover archive.
    Recover(Recover),
    /// Manage settings.
    #[clap(subcommand)]
    Settings(Settings),
}
