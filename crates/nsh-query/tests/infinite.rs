use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{InfiniteQueryOptions, RetryPolicy, use_infinite_query};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

#[derive(Clone)]
struct Page {
    number: u32,
    next: Option<u32>,
    previous: Option<u32>,
}

static PAGE_LOADS: AtomicUsize = AtomicUsize::new(0);

async fn load(number: u32) -> Result<Page, String> {
    timer(Duration::from_millis(10)).await;
    Ok(Page {
        number,
        next: (number < 3).then_some(number + 1),
        previous: (number > 1).then_some(number - 1),
    })
}

#[component]
pub fn PagesApp() -> impl IntoElement {
    let options = InfiniteQueryOptions::new(2_u32, |page: &Page| page.next)
        .get_previous(|page| page.previous);
    let pages = use_infinite_query::<u32, Page, String, _, _>("pages", options, |number, _| {
        PAGE_LOADS.fetch_add(1, Ordering::SeqCst);
        load(number)
    });
    view()
        .child(text(move || {
            let numbers: Vec<String> = pages.pages().iter().map(|p| p.number.to_string()).collect();
            format!("pages {}", numbers.join(","))
        }))
        .child(text(move || format!("more {}", pages.has_next_page())))
        .child(text(move || {
            format!("earlier {}", pages.has_previous_page())
        }))
        .child(text("next").on_press(move || pages.fetch_next_page()))
        .child(text("previous").on_press(move || pages.fetch_previous_page()))
        .child(text("refetch").on_press(move || pages.refetch()))
}

#[test]
fn pages_load_in_both_directions_and_stop_at_the_ends() {
    let mut app = start(Root::of::<PagesApp>());
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:pages 2").unwrap();
    assert!(app.shows("more true"));
    assert!(app.shows("earlier true"));

    app.press("text:next").unwrap();
    // Asking again while a page is on its way starts nothing.
    let before = PAGE_LOADS.load(Ordering::SeqCst);
    app.press("text:next").unwrap();
    assert_eq!(PAGE_LOADS.load(Ordering::SeqCst), before);
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:pages 2,3").unwrap();
    assert!(app.shows("more false"));

    app.press("text:next").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    assert!(app.shows("pages 2,3"));

    app.press("text:previous").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:pages 1,2,3").unwrap();
    assert!(app.shows("earlier false"));

    app.press("text:refetch").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:pages 2").unwrap();
}

#[component]
pub fn CappedApp() -> impl IntoElement {
    let options = InfiniteQueryOptions::new(1_u32, |page: &Page| page.next).max_pages(2);
    let pages =
        use_infinite_query::<u32, Page, String, _, _>("capped", options, |number, _| load(number));
    view()
        .child(text(move || {
            let numbers: Vec<String> = pages.pages().iter().map(|p| p.number.to_string()).collect();
            format!("kept {}", numbers.join(","))
        }))
        .child(text("next").on_press(move || pages.fetch_next_page()))
}

#[test]
fn max_pages_drops_from_the_far_end() {
    let mut app = start(Root::of::<CappedApp>());
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:kept 1").unwrap();
    app.press("text:next").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    app.press("text:next").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:kept 2,3").unwrap();
}

static FLAKY_PAGE: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FlakyPagesApp() -> impl IntoElement {
    let options =
        InfiniteQueryOptions::new(1_u32, |page: &Page| page.next).retry(RetryPolicy::no_retries());
    let pages = use_infinite_query::<u32, Page, String, _, _>(
        "flaky-pages",
        options,
        |number, _| async move {
            if number == 2 && FLAKY_PAGE.fetch_add(1, Ordering::SeqCst) == 0 {
                return Err("dropped".to_owned());
            }
            Ok(Page {
                number,
                next: Some(number + 1),
                previous: None,
            })
        },
    );
    view()
        .child(text(move || format!("loaded {}", pages.page_count())))
        .child(text(move || {
            format!("error {}", pages.error().unwrap_or_default())
        }))
        .child(text("next").on_press(move || pages.fetch_next_page()))
}

#[test]
fn a_failed_page_keeps_the_loaded_pages_and_can_be_asked_for_again() {
    let mut app = start(Root::of::<FlakyPagesApp>());
    app.wait_for("text:loaded 1").unwrap();
    app.press("text:next").unwrap();
    app.wait_for("text:error dropped").unwrap();
    assert!(app.shows("loaded 1"));
    app.press("text:next").unwrap();
    app.wait_for("text:loaded 2").unwrap();
}
