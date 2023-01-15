use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn html_to_md(html: &str) -> String {
    let mut cmd = Command::new("pandoc")
        .args(&[
            "-f",
            "html",
            "-t",
            "markdown_strict-raw_html",
            "--reference-links",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let mut i = cmd.stdin.take().unwrap();
        let _ = i.write(html.as_bytes()).await.unwrap();
    }

    let o = cmd.wait_with_output().await.unwrap().stdout;

    String::from_utf8(o).unwrap()
}
