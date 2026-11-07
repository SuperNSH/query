use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{InfiniteQueryOptions, use_infinite_query};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

#[derive(Clone)]
struct Doc {
    text: String,
    next: Option<u32>,
}

static VERSION: AtomicU32 = AtomicU32::new(1);
static REFETCH_LOADS: AtomicUsize = AtomicUsize::new(0);

async fn doc(number: u32) -> Result<Doc, String> {
    REFETCH_LOADS.fetch_add(1, Ordering::SeqCst);
    timer(Duration::from_millis(10)).await;
    Ok(Doc {
        text: format!("v{}-{number}", VERSION.load(Ordering::SeqCst)),
        next: (number < 3).then_some(number + 1),
    })
}

#[component]
pub fn RefetchApp() -> impl IntoElement {
    let options = InfiniteQueryOptions::new(1_u32, |d: &Doc| d.next);
    let pages =
        use_infinite_query::<u32, Doc, String, _, _>("refetch-docs", options, |n, _| doc(n));
    view()
        .child(text(move || {
            let texts: Vec<String> = pages.pages().iter().map(|d| d.text.clone()).collect();
            format!("pages {}", texts.join(","))
        }))
        .child(text("next").on_press(move || pages.fetch_next_page()))
        .child(text("bump").on_press(|| {
            VERSION.fetch_add(1, Ordering::SeqCst);
        }))
        .child(text("refetch").on_press(move || pages.refetch()))
        .child(text("reset").on_press(move || pages.reset()))
}

#[test]
fn infinite_refetch_reloads_all_pages() {
    let mut app = start(Root::of::<RefetchApp>());
    app.wait_for("text:pages v1-1").unwrap();
    app.press("text:next").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    app.press("text:next").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    app.wait_for("text:pages v1-1,v1-2,v1-3").unwrap();

    app.press("text:bump").unwrap();
    let before = REFETCH_LOADS.load(Ordering::SeqCst);
    app.press("text:refetch").unwrap();
    app.run_for(Duration::from_millis(40)).unwrap();
    // Every loaded page is fetched again, in order, from its own param.
    assert_eq!(REFETCH_LOADS.load(Ordering::SeqCst) - before, 3);
    app.wait_for("text:pages v2-1,v2-2,v2-3").unwrap();

    app.press("text:reset").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:pages v2-1").unwrap();
}

static SHARED_LOADS: AtomicUsize = AtomicUsize::new(0);

async fn shared_page(number: u32) -> Result<Doc, String> {
    SHARED_LOADS.fetch_add(1, Ordering::SeqCst);
    timer(Duration::from_millis(10)).await;
    Ok(Doc {
        text: format!("p{number}"),
        next: None,
    })
}

#[component]
pub fn SharedApp() -> impl IntoElement {
    let first = use_infinite_query::<u32, Doc, String, _, _>(
        "shared-list",
        InfiniteQueryOptions::new(1_u32, |d: &Doc| d.next),
        |n, _| shared_page(n),
    );
    let second = use_infinite_query::<u32, Doc, String, _, _>(
        "shared-list",
        InfiniteQueryOptions::new(1_u32, |d: &Doc| d.next),
        |n, _| shared_page(n),
    );
    view()
        .child(text(move || format!("first {}", first.page_count())))
        .child(text(move || format!("second {}", second.page_count())))
}

#[test]
fn infinite_shared_between_observers() {
    let mut app = start(Root::of::<SharedApp>());
    app.wait_for("text:first 1").unwrap();
    app.wait_for("text:second 1").unwrap();
    assert_eq!(SHARED_LOADS.load(Ordering::SeqCst), 1);
}

static REMOUNT_LOADS: AtomicUsize = AtomicUsize::new(0);

async fn remount_page(number: u32) -> Result<Doc, String> {
    REMOUNT_LOADS.fetch_add(1, Ordering::SeqCst);
    timer(Duration::from_millis(10)).await;
    Ok(Doc {
        text: format!("r{number}"),
        next: None,
    })
}

fn remount_child() -> impl IntoElement {
    let pages = use_infinite_query::<u32, Doc, String, _, _>(
        "remount-list",
        InfiniteQueryOptions::new(1_u32, |d: &Doc| d.next),
        |n, _| remount_page(n),
    );
    text(move || format!("remount {}", pages.page_count()))
}

#[component]
pub fn RemountApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(move || shown.get(), remount_child))
}

#[test]
fn infinite_survives_remount_within_gc() {
    let mut app = start(Root::of::<RemountApp>());
    app.wait_for("text:remount 1").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(Duration::from_secs(5)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:remount 1").unwrap();
    assert_eq!(REMOUNT_LOADS.load(Ordering::SeqCst), 1);
}
