use std::io::ErrorKind;
use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::ConvertError;

/// Converts HTML into Markdown using `pandoc`, which is required to be installed on the system.
pub async fn html_to_md(html: &str) -> Result<String, ConvertError> {
    let mut cmd = Command::new("pandoc")
        .args([
            "-f",
            "html",
            "-t",
            "markdown_strict-raw_html",
            "--wrap=none",
            "--reference-links",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                ConvertError::CommandNotFound("pandoc".to_string())
            } else {
                ConvertError::CommandError(e)
            }
        })?;

    {
        let mut i = cmd.stdin.take().expect("Could not obtain stdin");
        let _ = i.write(html.as_bytes()).await?;
    }

    let o = cmd.wait_with_output().await?.stdout;

    String::from_utf8(o).map_err(|e| ConvertError::CommandOutput(Box::new(e)))
}
