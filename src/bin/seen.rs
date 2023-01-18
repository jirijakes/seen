use std::path::PathBuf;

use clap::{Parser, Subcommand};
use comfy_table::{presets, Attribute, Cell, CellAlignment, Table};
use futures::StreamExt;
use isahc::http::Uri;
use miette::Result;
use seen::document::Content;
use seen::Seen;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let seen = Seen::new(&args.config).await?;

    match args.command {
        Command::Add(Add {
            url,
            tags,
            no_archive,
        }) => {
            seen::job::go(&seen, url, &tags, !no_archive).await?;
        }
        Command::Get(Get { uuid: id }) => {
            if let Ok(doc) = seen.get(&id).await {
                match doc.content {
                    Content::WebPage { text, rich_text } => {
                        if let Some(content) = rich_text {
                            let content = format!("# {}\n\n{}", doc.title, content);
                            display_content(&content).unwrap();
                        } else {
                            println!("{}\n\n{}", doc.title, text);
                        }
                    }
                };
            } else {
                println!("Not found.");
            }
        }
        Command::Search(Search { query }) => {
            futures::stream::iter(seen.search(&query)?)
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

                    table.add_row(vec![
                        Cell::new("Added")
                            .add_attribute(Attribute::Bold)
                            .set_alignment(CellAlignment::Right),
                        Cell::new(document.time.date()),
                    ]);

                    if let Some(tags) = document.metadata.get("tags") {
                        if let Ok(tags) = serde_json::from_value::<Vec<String>>(tags.clone()) {
                            if !tags.is_empty() {
                                table.add_row(vec![
                                    Cell::new("Tags")
                                        .add_attribute(Attribute::Bold)
                                        .set_alignment(CellAlignment::Right),
                                    Cell::new(tags.join(", ")),
                                ]);
                            }
                        }
                    }

                    table.add_row(vec![
                        Cell::new("UUID")
                            .add_attribute(Attribute::Bold)
                            .set_alignment(CellAlignment::Right),
                        Cell::new(hit.uuid.to_string()),
                    ]);

                    // table.add_row(vec![
                    //     Cell::new("Score")
                    //         .add_attribute(Attribute::Bold)
                    //         .set_alignment(CellAlignment::Right),
                    //     Cell::new(&hit.score.to_string()),
                    // ]);

                    if !hit.snippet.is_empty() {
                        table.add_row(vec![
                            Cell::new("Snippet")
                                .add_attribute(Attribute::Bold)
                                .set_alignment(CellAlignment::Right),
                            Cell::new(termimad::term_text(&hit.snippet)),
                        ]);
                    }

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

    /// Do not archive this source.
    #[arg(long, default_value = "false")]
    no_archive: bool,
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

use std::io::{stdout, Write};

use termimad::crossterm::cursor::{Hide, Show};
use termimad::crossterm::event::KeyCode::*;
use termimad::crossterm::event::{self, Event, KeyEvent};
use termimad::crossterm::queue;
use termimad::crossterm::style::Color::*;
use termimad::crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use termimad::*;

//
// Taken from termimad examples. Thank you!
//

fn display_content(content: &str) -> Result<(), Error> {
    let skin = make_skin();
    let mut w = stdout(); // we could also have used stderr
    queue!(w, EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    queue!(w, Hide)?; // hiding the cursor
    let mut view = MadView::from(content.to_owned(), view_area(), skin);
    loop {
        view.write_on(&mut w)?;
        w.flush()?;
        match event::read() {
            Ok(Event::Key(KeyEvent { code, .. })) => match code {
                Up => view.try_scroll_lines(-1),
                Down => view.try_scroll_lines(1),
                PageUp => view.try_scroll_pages(-1),
                PageDown => view.try_scroll_pages(1),
                _ => break,
            },
            Ok(Event::Resize(..)) => {
                queue!(w, Clear(ClearType::All))?;
                view.resize(&view_area());
            }
            _ => {}
        }
    }
    terminal::disable_raw_mode()?;
    queue!(w, Show)?; // we must restore the cursor
    queue!(w, LeaveAlternateScreen)?;
    w.flush()?;
    Ok(())
}

fn view_area() -> Area {
    let mut area = Area::full_screen();
    area.pad_for_max_width(100);
    area
}

fn make_skin() -> MadSkin {
    let mut skin = MadSkin::default();
    skin.table.align = Alignment::Center;
    skin.set_headers_fg(AnsiValue(178));
    skin.bold.set_fg(Yellow);
    skin.italic.set_fg(Magenta);
    skin.scrollbar.thumb.set_fg(AnsiValue(178));
    skin.code_block.align = Alignment::Center;
    skin
}
