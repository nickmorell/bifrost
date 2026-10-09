//! Temporary: fetches a Bandcamp page the way BiFrost does and says what it got.
use bifrost::net::{Method, Request, Transport, shared};

#[tokio::main]
async fn main() {
    let url = std::env::args().nth(1).expect("give the Bandcamp link");
    let req = Request {
        method: Method::Get,
        url: url::Url::parse(&url).expect("a link"),
        headers: Vec::new(),
        body: None,
        timeout: None,
        limit: bifrost::net::MAX_RESPONSE,
    };
    match shared().send(req).await {
        Ok(r) => {
            println!("status {} bytes {} moved_to {:?}", r.status, r.body.len(), r.moved_to);
            for (k, v) in &r.headers {
                println!("  {k}: {v}");
            }
            std::fs::write("bc-app.html", &r.body).ok();
            let html = String::from_utf8_lossy(&r.body);
            println!("has data-tralbum: {}", html.contains("data-tralbum=\""));
            let rel = bifrost::integrations::bandcamp::release(&html);
            println!("release: {:?}", rel.map(|r| (r.title, r.tracks.len())));
        }
        Err(e) => println!("failed: {e}"),
    }
}
