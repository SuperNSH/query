use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    CachePolicy, QueryOptions, RetryPolicy, query_client, use_query, use_query_by, use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

fn seconds(n: u64) -> Duration {
    Duration::from_secs(n)
}

static SHARED_LOADS: AtomicUsize = AtomicUsize::new(0);

fn shared_row(label: &'static str) -> impl IntoElement {
    let query = use_query::<String, String, _, _>("shared", |_| async {
        SHARED_LOADS.fetch_add(1, Ordering::SeqCst);
        timer(Duration::from_millis(50)).await;
        Ok("loaded".to_owned())
    });
    text(move || {
        format!(
            "{label}: {}",
            query.data().unwrap_or_else(|| "pending".to_owned())
        )
    })
}

#[component]
pub fn SharedApp() -> impl IntoElement {
    view()
        .child(shared_row("first"))
        .child(shared_row("second"))
}

#[test]
fn two_observers_of_one_key_share_one_load() {
    let mut app = start(Root::of::<SharedApp>());
    app.wait_for("text:first: pending").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:first: loaded").unwrap();
    app.wait_for("text:second: loaded").unwrap();
    assert_eq!(SHARED_LOADS.load(Ordering::SeqCst), 1);
}

static CACHE_LOADS: AtomicUsize = AtomicUsize::new(0);

fn cached_child() -> impl IntoElement {
    let options = QueryOptions::new().cache(CachePolicy::Ttl { ttl_ms: 60_000 });
    let query = use_query_with::<usize, String, _, _>("cached", options, |_| async {
        Ok(CACHE_LOADS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("load {}", query.data().unwrap_or(0)))
}

#[component]
pub fn CacheApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(move || shown.get(), cached_child))
}

#[test]
fn a_fresh_entry_skips_the_load_and_an_expired_one_reloads() {
    let mut app = start(Root::of::<CacheApp>());
    app.wait_for("text:load 1").unwrap();

    app.press("text:toggle").unwrap();
    app.wait_for_gone("text:load 1", Duration::from_secs(1))
        .unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:load 1").unwrap();
    assert_eq!(CACHE_LOADS.load(Ordering::SeqCst), 1);

    app.press("text:toggle").unwrap();
    app.run_for(seconds(61)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:load 2").unwrap();
    assert_eq!(CACHE_LOADS.load(Ordering::SeqCst), 2);
}

static RETRY_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn RetryApp() -> impl IntoElement {
    let options = QueryOptions::new().retry(RetryPolicy::new(2).with_delay(1_000));
    let query = use_query_with::<String, String, _, _>("flaky", options, |_| async {
        let attempt = RETRY_ATTEMPTS.fetch_add(1, Ordering::SeqCst);
        if attempt < 2 {
            Err("nope".to_owned())
        } else {
            Ok("worked".to_owned())
        }
    });
    view()
        .child(text(move || {
            format!("data {}", query.data().unwrap_or_default())
        }))
        .child(text(move || format!("failed {}", query.is_failure())))
        .child(text(move || format!("retries {}", query.retry_count())))
}

#[test]
fn retries_stay_loading_and_end_in_success() {
    let mut app = start(Root::of::<RetryApp>());
    app.wait_for("text:retries 1").unwrap();
    assert!(app.shows("failed false"));
    app.run_for(Duration::from_millis(1_000)).unwrap();
    app.wait_for("text:retries 2").unwrap();
    app.run_for(Duration::from_millis(1_000)).unwrap();
    app.wait_for("text:data worked").unwrap();
    assert!(app.shows("failed false"));
    assert!(app.shows("retries 0"));
}

#[component]
pub fn FailApp() -> impl IntoElement {
    let options = QueryOptions::new().retry(RetryPolicy::no_retries());
    let query = use_query_with::<String, String, _, _>("broken", options, |_| async {
        Err("offline".to_owned())
    });
    text(move || format!("error {}", query.error().unwrap_or_default()))
}

#[test]
fn a_failure_with_no_retries_left_reports_the_error() {
    let mut app = start(Root::of::<FailApp>());
    app.wait_for("text:error offline").unwrap();
}

static VERSION: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn InvalidateApp() -> impl IntoElement {
    let query = use_query::<usize, String, _, _>(["todos", "list"], |_| async {
        let version = VERSION.fetch_add(1, Ordering::SeqCst) + 1;
        // The first load is slow, so a later one overtakes it.
        timer(Duration::from_millis(if version == 1 { 1_000 } else { 10 })).await;
        Ok(version)
    });
    view()
        .child(text(move || {
            format!("version {}", query.data().unwrap_or(0))
        }))
        .child(text("invalidate").on_press(|| query_client().invalidate("todos")))
}

#[test]
fn invalidating_replaces_a_load_in_flight_and_drops_its_late_answer() {
    let mut app = start(Root::of::<InvalidateApp>());
    app.wait_for("text:version 0").unwrap();
    app.press("text:invalidate").unwrap();
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:version 2").unwrap();
    app.run_for(Duration::from_millis(2_000)).unwrap();
    assert!(app.shows("version 2"));
}

static BY_KEY_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ByKeyApp() -> impl IntoElement {
    let id = signal(1_u32);
    let query = use_query_by::<String, String, _, _, _>(
        move || gpui_key(id.get()),
        QueryOptions::new(),
        |context| async move {
            BY_KEY_LOADS.fetch_add(1, Ordering::SeqCst);
            Ok(format!("user {}", context.key.parts()[1]))
        },
    );
    view()
        .child(text(move || {
            query.data().unwrap_or_else(|| "none".to_owned())
        }))
        .child(text("two").on_press(move || id.set(2)))
        .child(text("one").on_press(move || id.set(1)))
}

fn gpui_key(id: u32) -> nsh_query::QueryKey {
    nsh_query::QueryKey::new(["users".to_owned(), id.to_string()])
}

#[test]
fn a_tracked_key_moves_to_the_new_entry_and_back_to_its_cache() {
    let mut app = start(Root::of::<ByKeyApp>());
    app.wait_for("text:user 1").unwrap();
    app.press("text:two").unwrap();
    app.wait_for("text:user 2").unwrap();
    app.press("text:one").unwrap();
    app.wait_for("text:user 1").unwrap();
    assert_eq!(BY_KEY_LOADS.load(Ordering::SeqCst), 2);
}

#[component]
pub fn EnabledApp() -> impl IntoElement {
    let on = signal(false);
    let options = QueryOptions::new().enabled(move || on.get());
    let query = use_query_with::<String, String, _, _>("gated", options, |_| async {
        Ok("ran".to_owned())
    });
    view()
        .child(text(move || {
            format!(
                "state {}",
                query.data().unwrap_or_else(|| "idle".to_owned())
            )
        }))
        .child(text("enable").on_press(move || on.set(true)))
}

#[test]
fn a_disabled_query_waits_for_its_condition() {
    let mut app = start(Root::of::<EnabledApp>());
    app.wait_for("text:state idle").unwrap();
    app.press("text:enable").unwrap();
    app.wait_for("text:state ran").unwrap();
}

static SET_DATA_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn SetDataApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<String, String>("seeded", "from cache".to_owned());
    let query = use_query::<String, String, _, _>("seeded", |_| async {
        SET_DATA_LOADS.fetch_add(1, Ordering::SeqCst);
        Ok("from network".to_owned())
    });
    text(move || query.data().unwrap_or_default())
}

#[test]
fn seeded_data_counts_as_fresh() {
    let mut app = start(Root::of::<SetDataApp>());
    app.wait_for("text:from cache").unwrap();
    assert_eq!(SET_DATA_LOADS.load(Ordering::SeqCst), 0);
}

static GC_LOADS: AtomicUsize = AtomicUsize::new(0);

fn gc_child() -> impl IntoElement {
    let options = QueryOptions::new().ttl(seconds(3_600)).gc(seconds(10));
    let query = use_query_with::<usize, String, _, _>("collected", options, |_| async {
        Ok(GC_LOADS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("copy {}", query.data().unwrap_or(0)))
}

#[component]
pub fn GcApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(move || shown.get(), gc_child))
}

#[test]
fn an_unobserved_entry_is_dropped_after_its_gc_time() {
    let mut app = start(Root::of::<GcApp>());
    app.wait_for("text:copy 1").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(seconds(5)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:copy 1").unwrap();
    assert_eq!(GC_LOADS.load(Ordering::SeqCst), 1);

    app.press("text:toggle").unwrap();
    app.run_for(seconds(11)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:copy 2").unwrap();
    assert_eq!(GC_LOADS.load(Ordering::SeqCst), 2);
}

static TICKS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn IntervalApp() -> impl IntoElement {
    let options = QueryOptions::new().refetch_interval(seconds(30));
    let query = use_query_with::<usize, String, _, _>("ticking", options, |_| async {
        Ok(TICKS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("tick {}", query.data().unwrap_or(0)))
}

#[test]
fn a_refetch_interval_keeps_loading_while_observed() {
    let mut app = start(Root::of::<IntervalApp>());
    app.wait_for("text:tick 1").unwrap();
    app.run_for(seconds(31)).unwrap();
    app.wait_for("text:tick 2").unwrap();
    app.run_for(seconds(30)).unwrap();
    app.wait_for("text:tick 3").unwrap();
}
