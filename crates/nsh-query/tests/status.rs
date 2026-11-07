use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::platform::Lifecycle;
use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    QueryConfig, QueryKey, QueryOptions, RefetchTrigger, RetryPolicy, use_query, use_query_config,
    use_query_with,
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

#[component]
pub fn LoadApp() -> impl IntoElement {
    let query = use_query::<String, String, _, _>("status-load", |_| async {
        timer(Duration::from_millis(100)).await;
        Ok("done".to_owned())
    });
    view()
        .child(text(move || format!("status {:?}", query.status_v5())))
        .child(text(move || format!("fetch {:?}", query.fetch_status())))
}

#[test]
fn status_and_fetch_status_follow_a_load() {
    let mut app = start(Root::of::<LoadApp>());
    app.wait_for("text:status Pending").unwrap();
    app.wait_for("text:fetch Fetching").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:status Success").unwrap();
    assert!(app.shows("fetch Idle"));
}

#[component]
pub fn FlagsApp() -> impl IntoElement {
    let query = use_query::<String, String, _, _>("flags", |_| async {
        timer(Duration::from_millis(100)).await;
        Ok("done".to_owned())
    });
    view()
        .child(text(move || {
            format!(
                "flags {} {} {}",
                query.is_loading(),
                query.is_fetching(),
                query.is_pending()
            )
        }))
        .child(text(move || {
            format!("refetching {}", query.is_refetching())
        }))
        .child(text("refetch").on_press(move || query.refetch()))
}

#[test]
fn is_loading_is_fetching_and_pending() {
    let mut app = start(Root::of::<FlagsApp>());
    app.wait_for("text:flags true true true").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:flags false false false").unwrap();
    app.press("text:refetch").unwrap();
    app.wait_for("text:refetching true").unwrap();
    assert!(app.shows("flags false true false"));
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:refetching false").unwrap();
}

static RETRY_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FailuresApp() -> impl IntoElement {
    let options = QueryOptions::new().retry(RetryPolicy::new(2).with_delay(1_000));
    let query = use_query_with::<String, String, _, _>("failures", options, |_| async {
        let attempt = RETRY_LOADS.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt <= 2 {
            Err(format!("attempt {attempt}"))
        } else {
            Ok("ok".to_owned())
        }
    });
    view()
        .child(text(move || format!("failures {}", query.failure_count())))
        .child(text(move || {
            format!(
                "reason {}",
                query.failure_reason().unwrap_or_else(|| "none".to_owned())
            )
        }))
        .child(text(move || {
            format!("data {}", query.data().unwrap_or_default())
        }))
}

#[test]
fn failure_count_and_reason_track_retries() {
    let mut app = start(Root::of::<FailuresApp>());
    app.wait_for("text:failures 1").unwrap();
    assert!(app.shows("reason attempt 1"));
    app.run_for(seconds(1)).unwrap();
    app.wait_for("text:failures 2").unwrap();
    assert!(app.shows("reason attempt 2"));
    app.run_for(seconds(1)).unwrap();
    app.wait_for("text:data ok").unwrap();
    assert!(app.shows("failures 0"));
    assert!(app.shows("reason none"));
}

static RETRY_IF_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn RetryIfApp() -> impl IntoElement {
    let config = QueryConfig::<String, String>::from(QueryOptions::new())
        .retry_if(|count, error: &String| error != "fatal" && count < 5);
    let query = use_query_config::<String, String, _, _>("retry-if", config, |_| async {
        RETRY_IF_LOADS.fetch_add(1, Ordering::SeqCst);
        Err("fatal".to_owned())
    });
    text(move || format!("error {}", query.error().unwrap_or_default()))
}

#[test]
fn retry_if_stops_early() {
    let mut app = start(Root::of::<RetryIfApp>());
    app.wait_for("text:error fatal").unwrap();
    app.run_for(seconds(10)).unwrap();
    assert_eq!(RETRY_IF_LOADS.load(Ordering::SeqCst), 1);
}

static DELAY_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn RetryDelayApp() -> impl IntoElement {
    let config =
        QueryConfig::<String, String>::from(QueryOptions::new().retry(RetryPolicy::new(3)))
            .retry_delay(|count, _: &String| Duration::from_millis(250 * (u64::from(count) + 1)));
    let query = use_query_config::<String, String, _, _>("retry-delay", config, |_| async {
        let attempt = DELAY_LOADS.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt < 3 {
            Err("again".to_owned())
        } else {
            Ok("third".to_owned())
        }
    });
    text(move || format!("data {}", query.data().unwrap_or_default()))
}

#[test]
fn retry_delay_function_is_honored() {
    // TanStack passes the failures before this one (0 at first), so the first retry waits 250 ms, then 500 ms.
    let mut app = start(Root::of::<RetryDelayApp>());
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(DELAY_LOADS.load(Ordering::SeqCst), 1);
    app.run_for(Duration::from_millis(200)).unwrap();
    assert_eq!(DELAY_LOADS.load(Ordering::SeqCst), 1);
    app.run_for(Duration::from_millis(100)).unwrap();
    assert_eq!(DELAY_LOADS.load(Ordering::SeqCst), 2);
    app.run_for(Duration::from_millis(400)).unwrap();
    assert_eq!(DELAY_LOADS.load(Ordering::SeqCst), 2);
    app.run_for(Duration::from_millis(200)).unwrap();
    app.wait_for("text:data third").unwrap();
    assert_eq!(DELAY_LOADS.load(Ordering::SeqCst), 3);
}

static SYNONYM_LOADS: AtomicUsize = AtomicUsize::new(0);

fn synonym_child() -> impl IntoElement {
    let options = QueryOptions::new()
        .stale_time(seconds(60))
        .gc_time(seconds(10));
    let query = use_query_with::<usize, String, _, _>("synonyms", options, |_| async {
        Ok(SYNONYM_LOADS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("copy {}", query.data().unwrap_or(0)))
}

#[component]
pub fn SynonymApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(move || shown.get(), synonym_child))
}

#[test]
fn stale_time_and_gc_time_synonyms_work() {
    let mut app = start(Root::of::<SynonymApp>());
    app.wait_for("text:copy 1").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(seconds(5)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:copy 1").unwrap();
    assert_eq!(SYNONYM_LOADS.load(Ordering::SeqCst), 1);
    app.press("text:toggle").unwrap();
    app.run_for(seconds(11)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:copy 2").unwrap();
    assert_eq!(SYNONYM_LOADS.load(Ordering::SeqCst), 2);
}

static SEED_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn SeedApp() -> impl IntoElement {
    let now = nsh_core::time::unix_ms() as u64;
    let fresh = QueryConfig::<String, String>::from(
        QueryOptions::new()
            .stale_time(seconds(60))
            .initial_data_updated_at(now),
    )
    .initial_data(|| "seed".to_owned());
    let old = QueryConfig::<String, String>::from(
        QueryOptions::new()
            .stale_time(seconds(60))
            .initial_data_updated_at(now - 120_000),
    )
    .initial_data(|| "seed".to_owned());
    let fresh_q = use_query_config::<String, String, _, _>("seed-fresh", fresh, |_| async {
        SEED_LOADS.fetch_add(1, Ordering::SeqCst);
        Ok("network".to_owned())
    });
    let old_q = use_query_config::<String, String, _, _>("seed-old", old, |_| async {
        SEED_LOADS.fetch_add(1, Ordering::SeqCst);
        Ok("network".to_owned())
    });
    view()
        .child(text(move || {
            format!("fresh {}", fresh_q.data().unwrap_or_default())
        }))
        .child(text(move || {
            format!("old {}", old_q.data().unwrap_or_default())
        }))
}

#[test]
fn initial_data_counts_as_fresh_with_updated_at() {
    let mut app = start(Root::of::<SeedApp>());
    // Only the stale seed refetches on mount; the fresh one keeps its seed.
    app.wait_for("text:old network").unwrap();
    assert!(app.shows("fresh seed"));
    assert_eq!(SEED_LOADS.load(Ordering::SeqCst), 1);
}

#[component]
pub fn PlaceholderApp() -> impl IntoElement {
    let id = signal(1_u32);
    let placeholder = QueryConfig::<String, String>::from(QueryOptions::new())
        .placeholder_data(|_| Some("placeholder".to_owned()));
    let keep = QueryOptions::new().keep_previous_data();
    let with_placeholder = nsh_query::use_query_by_config::<String, String, _, _, _>(
        move || QueryKey::new(["ph".to_owned(), id.get().to_string()]),
        placeholder,
        |context| async move {
            timer(Duration::from_millis(100)).await;
            Ok(format!("item {}", context.key.parts()[1]))
        },
    );
    let kept = nsh_query::use_query_by::<String, String, _, _, _>(
        move || QueryKey::new(["kp".to_owned(), id.get().to_string()]),
        keep,
        |context| async move {
            timer(Duration::from_millis(100)).await;
            Ok(format!("item {}", context.key.parts()[1]))
        },
    );
    view()
        .child(text(move || {
            format!(
                "ph {} {}",
                with_placeholder.data().unwrap_or_else(|| "none".to_owned()),
                with_placeholder.is_placeholder_data()
            )
        }))
        .child(text(move || {
            format!(
                "kp {} {}",
                kept.data().unwrap_or_else(|| "none".to_owned()),
                kept.is_placeholder_data()
            )
        }))
        .child(text("two").on_press(move || id.set(2)))
}

#[test]
fn placeholder_data_and_keep_previous_data_flag_placeholder() {
    let mut app = start(Root::of::<PlaceholderApp>());
    app.wait_for("text:ph placeholder true").unwrap();
    assert!(app.shows("kp none false"));
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:ph item 1 false").unwrap();
    app.wait_for("text:kp item 1 false").unwrap();

    app.press("text:two").unwrap();
    app.wait_for("text:ph placeholder true").unwrap();
    app.wait_for("text:kp item 1 true").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:kp item 2 false").unwrap();
    app.wait_for("text:ph item 2 false").unwrap();
}

static MOUNT_ALWAYS: AtomicUsize = AtomicUsize::new(0);
static MOUNT_LAZY: AtomicUsize = AtomicUsize::new(0);

fn mount_child() -> impl IntoElement {
    let always = use_query_with::<usize, String, _, _>(
        "mount-always",
        QueryOptions::new()
            .stale_time(seconds(60))
            .refetch_on_mount(RefetchTrigger::Always),
        |_| async { Ok(MOUNT_ALWAYS.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    let lazy = use_query_with::<usize, String, _, _>(
        "mount-lazy",
        QueryOptions::new().stale_time(seconds(60)),
        |_| async { Ok(MOUNT_LAZY.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    view()
        .child(text(move || {
            format!("always {}", always.data().unwrap_or(0))
        }))
        .child(text(move || format!("lazy {}", lazy.data().unwrap_or(0))))
}

#[component]
pub fn MountApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(move || shown.get(), mount_child))
}

#[test]
fn refetch_on_mount_trigger_is_honored() {
    let mut app = start(Root::of::<MountApp>());
    app.wait_for("text:always 1").unwrap();
    app.wait_for("text:lazy 1").unwrap();
    app.press("text:toggle").unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:always 2").unwrap();
    assert!(app.shows("lazy 1"));
    assert_eq!(MOUNT_LAZY.load(Ordering::SeqCst), 1);
}

static TICKS_BG: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn BackgroundTickApp() -> impl IntoElement {
    let options = QueryOptions::new().refetch_interval(seconds(30));
    let query = use_query_with::<usize, String, _, _>("bg-tick", options, |_| async {
        Ok(TICKS_BG.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("tick {}", query.data().unwrap_or(0)))
}

#[test]
fn refetch_interval_stops_in_background() {
    let mut app = start(Root::of::<BackgroundTickApp>());
    app.wait_for("text:tick 1").unwrap();
    app.set_environment("background", |e| e.lifecycle = Lifecycle::Background)
        .unwrap();
    app.run_for(seconds(31)).unwrap();
    assert_eq!(TICKS_BG.load(Ordering::SeqCst), 1);
    app.set_environment("active", |e| e.lifecycle = Lifecycle::Active)
        .unwrap();
    app.run_for(seconds(30)).unwrap();
    app.wait_for("text:tick 2").unwrap();
}

static TICKS_INACTIVE: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn InactiveTickApp() -> impl IntoElement {
    let options = QueryOptions::new().refetch_interval(seconds(30));
    let query = use_query_with::<usize, String, _, _>("inactive-tick", options, |_| async {
        Ok(TICKS_INACTIVE.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("tick {}", query.data().unwrap_or(0)))
}

#[test]
fn refetch_interval_keeps_running_while_inactive() {
    let mut app = start(Root::of::<InactiveTickApp>());
    app.wait_for("text:tick 1").unwrap();
    app.set_environment("inactive", |e| e.lifecycle = Lifecycle::Inactive)
        .unwrap();
    app.run_for(seconds(30)).unwrap();
    app.wait_for("text:tick 2").unwrap();
}

static FOCUS_STALE: AtomicUsize = AtomicUsize::new(0);
static FOCUS_FRESH: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FocusApp() -> impl IntoElement {
    let stale = use_query_with::<usize, String, _, _>(
        "focus-stale",
        QueryOptions::new().ttl(seconds(60)),
        |_| async { Ok(FOCUS_STALE.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    let fresh = use_query_with::<usize, String, _, _>(
        "focus-fresh",
        QueryOptions::new().ttl(seconds(120)),
        |_| async { Ok(FOCUS_FRESH.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    view()
        .child(text(move || format!("stale {}", stale.data().unwrap_or(0))))
        .child(text(move || format!("fresh {}", fresh.data().unwrap_or(0))))
}

#[test]
fn refetch_on_foreground_when_stale() {
    let mut app = start(Root::of::<FocusApp>());
    app.wait_for("text:stale 1").unwrap();
    app.set_environment("background", |e| e.lifecycle = Lifecycle::Background)
        .unwrap();
    app.run_for(seconds(61)).unwrap();
    app.set_environment("active", |e| e.lifecycle = Lifecycle::Active)
        .unwrap();
    app.wait_for("text:stale 2").unwrap();
    assert!(app.shows("fresh 1"));
    assert_eq!(FOCUS_FRESH.load(Ordering::SeqCst), 1);
}
