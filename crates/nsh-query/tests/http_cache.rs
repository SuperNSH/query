#![cfg(feature = "http-cache")]
//! A query whose fetcher reads through `gpui-query-http`'s HTTP cache, with a fake backend.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{HttpBackend, HttpCache, QueryOptions, use_query_with};
use nsh_testing::Headless;

static BACKEND_CALLS: AtomicUsize = AtomicUsize::new(0);

struct FakeBackend;

#[derive(Debug)]
struct FakeError;

impl std::fmt::Display for FakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("fake backend failed")
    }
}

impl std::error::Error for FakeError {}

impl HttpBackend for FakeBackend {
    type Error = FakeError;

    fn fetch(
        &self,
        _url: &str,
        _conditionals: gpui_query_http::Conditionals,
    ) -> impl std::future::Future<Output = Result<gpui_query_http::BackendResponse, FakeError>>
    {
        BACKEND_CALLS.fetch_add(1, Ordering::SeqCst);
        async {
            Ok(gpui_query_http::BackendResponse {
                status: 200,
                headers: Default::default(),
                body: Bytes::from_static(b"hello from the fake backend"),
            })
        }
    }
}

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

#[component]
pub fn HttpApp() -> impl IntoElement {
    let cache = Rc::new(HttpCache::new(FakeBackend));
    let body =
        use_query_with::<String, String, _, _>("http-body", QueryOptions::new(), move |_| {
            let cache = cache.clone();
            async move {
                let (bytes, _policy, _meta) = cache
                    .fetch("https://example.test/data")
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            }
        });
    text(move || format!("body {}", body.data().unwrap_or_default()))
}

#[test]
fn http_cache_feeds_a_query_through_a_fake_backend() {
    // The query's fetcher reads the body through HttpCache; the backend runs once.
    let mut app = start(Root::of::<HttpApp>());
    app.wait_for("text:body hello from the fake backend")
        .unwrap();
    assert_eq!(BACKEND_CALLS.load(Ordering::SeqCst), 1);
}
