use chromiumoxide::cdp::browser_protocol::page::{CaptureScreenshotParams, PrintToPdfParams};
use chromiumoxide::{Browser, BrowserConfig};
use futures::StreamExt;

pub async fn pdf() {
    let (browser, mut handler) = Browser::launch(BrowserConfig::builder().build().unwrap())
        .await
        .unwrap();

    let handle = tokio::task::spawn(async move {
        loop {
            let _ = handler.next().await.unwrap();
        }
    });

    let page = browser.new_page("...").await.unwrap();

    page.save_pdf(PrintToPdfParams::default(), "out.pdf")
        .await
        .unwrap();

    page.save_screenshot(CaptureScreenshotParams::default(), "out.png")
        .await
        .unwrap();

    handle.await.unwrap();
}
