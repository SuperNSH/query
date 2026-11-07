//! Tests for the TanStack surface that had no direct test. Each behavior is
//! checked against `packages/query-core/src` in the TanStack reference.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::JsonValue;
use nsh_core::platform::Lifecycle;
use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    FetchQueryError, InfiniteQueryOptions, MutationDefaults, MutationOptions, NetworkMode,
    QueryKey, QueryOptions, RefetchTrigger, RetryPolicy, online_manager, query_client,
    use_infinite_query, use_mutation_with, use_mutation_with_context, use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn seconds(n: u64) -> Duration {
    Duration::from_secs(n)
}

// ---- Query result fields -------------------------------------------------

#[component]
pub fn InitialApp() -> impl IntoElement {
    let q =
        use_query_with::<String, String, _, _>("surface-init", QueryOptions::new(), |_| async {
            timer(ms(50)).await;
            Ok("v".to_owned())
        });
    view()
        .child(text(move || {
            format!(
                "init {} {} {}",
                q.is_initial_loading(),
                q.is_fetched_after_mount(),
                q.is_loading()
            )
        }))
        .child(text(move || {
            format!("data {}", q.data().unwrap_or_default())
        }))
}

#[test]
fn is_initial_loading_and_fetched_after_mount_follow_tanstack() {
    // TanStack: isInitialLoading is isLoading; isFetchedAfterMount is set once data settles.
    let mut app = start(Root::of::<InitialApp>());
    app.wait_for("text:init true false true").unwrap();
    app.run_for(ms(50)).unwrap();
    app.wait_for("text:data v").unwrap();
    assert!(app.shows("init false true false"));
}

#[component]
pub fn SuccessApp() -> impl IntoElement {
    let n = signal(0_u32);
    let q = use_query_with::<u32, String, _, _>(
        "surface-success",
        QueryOptions::new(),
        move |_| async move {
            timer(ms(50)).await;
            Ok(n.get_untracked() + 1)
        },
    );
    view()
        .child(text(move || {
            format!(
                "st {} {} {}",
                q.is_pending(),
                q.is_success(),
                q.is_failure()
            )
        }))
        .child(text(move || format!("fe {}", q.is_fetching())))
        .child(text("refetch").on_press(move || q.refetch()))
}

#[test]
fn is_success_and_is_pending_follow_tanstack_status() {
    // TanStack: isSuccess is status === 'success', and a refetch keeps that status.
    let mut app = start(Root::of::<SuccessApp>());
    app.wait_for("text:st true false false").unwrap();
    app.run_for(ms(50)).unwrap();
    app.wait_for("text:st false true false").unwrap();
    app.press("text:refetch").unwrap();
    app.wait_for("text:fe true").unwrap();
    assert!(app.shows("st false true false"));
    app.run_for(ms(50)).unwrap();
    app.wait_for("text:fe false").unwrap();
}

static WINDOW_A: AtomicUsize = AtomicUsize::new(0);
static WINDOW_B: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn WindowApp() -> impl IntoElement {
    let a = use_query_with::<usize, String, _, _>(
        "surface-window-a",
        QueryOptions::new().refetch_on_window_focus(RefetchTrigger::IfStale),
        |_| async { Ok(WINDOW_A.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    let b = use_query_with::<usize, String, _, _>(
        "surface-window-b",
        QueryOptions::new().refetch_on_window_focus(RefetchTrigger::Never),
        |_| async { Ok(WINDOW_B.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    view()
        .child(text(move || format!("a {}", a.data().unwrap_or(0))))
        .child(text(move || format!("b {}", b.data().unwrap_or(0))))
}

#[test]
fn refetch_on_window_focus_alias_sets_the_foreground_trigger() {
    let mut app = start(Root::of::<WindowApp>());
    app.wait_for("text:a 1").unwrap();
    app.set_environment("background", |e| e.lifecycle = Lifecycle::Background)
        .unwrap();
    app.run_for(seconds(61)).unwrap();
    app.set_environment("active", |e| e.lifecycle = Lifecycle::Active)
        .unwrap();
    app.wait_for("text:a 2").unwrap();
    assert!(app.shows("b 1"));
}

#[component]
pub fn MetaApp() -> impl IntoElement {
    let options = QueryOptions::new().meta(JsonValue::String("tagged".to_owned()));
    let q = use_query_with::<String, String, _, _>("surface-meta", options, |context| async move {
        Ok(context
            .meta
            .as_ref()
            .and_then(JsonValue::as_str)
            .unwrap_or("none")
            .to_owned())
    });
    text(move || format!("data {}", q.data().unwrap_or_default()))
}

#[test]
fn meta_reaches_the_fetcher_context() {
    // TanStack: queryFn receives the query's meta in its context.
    let mut app = start(Root::of::<MetaApp>());
    app.wait_for("text:data tagged").unwrap();
}

static TICKS_FN: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn IntervalFnApp() -> impl IntoElement {
    let options = QueryOptions::new().refetch_interval_fn(|| {
        if TICKS_FN.load(Ordering::SeqCst) < 3 {
            Some(seconds(10))
        } else {
            None
        }
    });
    let q = use_query_with::<usize, String, _, _>("surface-interval-fn", options, |_| async {
        Ok(TICKS_FN.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("ticks {}", q.data().unwrap_or(0)))
}

#[test]
fn refetch_interval_fn_stops_when_it_returns_none() {
    // TanStack: refetchInterval as a function; returning false stops the interval.
    let mut app = start(Root::of::<IntervalFnApp>());
    app.wait_for("text:ticks 1").unwrap();
    app.run_for(seconds(10)).unwrap();
    app.wait_for("text:ticks 2").unwrap();
    app.run_for(seconds(10)).unwrap();
    app.wait_for("text:ticks 3").unwrap();
    app.run_for(seconds(100)).unwrap();
    assert_eq!(TICKS_FN.load(Ordering::SeqCst), 3);
}

static BG_TICKS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn BackgroundIntervalApp() -> impl IntoElement {
    let options = QueryOptions::new()
        .refetch_interval(seconds(30))
        .refetch_interval_in_background(true);
    let q = use_query_with::<usize, String, _, _>("surface-bg-interval", options, |_| async {
        Ok(BG_TICKS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    text(move || format!("tick {}", q.data().unwrap_or(0)))
}

#[test]
fn refetch_interval_in_background_keeps_ticking() {
    // TanStack: refetchIntervalInBackground true refetches while the app is not focused.
    let mut app = start(Root::of::<BackgroundIntervalApp>());
    app.wait_for("text:tick 1").unwrap();
    app.set_environment("background", |e| e.lifecycle = Lifecycle::Background)
        .unwrap();
    app.run_for(seconds(31)).unwrap();
    app.wait_for("text:tick 2").unwrap();
    assert_eq!(BG_TICKS.load(Ordering::SeqCst), 2);
}

static ROFF: AtomicUsize = AtomicUsize::new(0);
static RON: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn RetryOnMountApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(
            move || shown.get(),
            || {
                let off = use_query_with::<usize, String, _, _>(
                    "surface-rom-off",
                    QueryOptions::new()
                        .retry(RetryPolicy::no_retries())
                        .retry_on_mount(false),
                    |_| async {
                        ROFF.fetch_add(1, Ordering::SeqCst);
                        Err::<usize, String>("down".to_owned())
                    },
                );
                let on = use_query_with::<usize, String, _, _>(
                    "surface-rom-on",
                    QueryOptions::new().retry(RetryPolicy::no_retries()),
                    |_| async {
                        RON.fetch_add(1, Ordering::SeqCst);
                        Err::<usize, String>("down".to_owned())
                    },
                );
                view()
                    .child(text(move || {
                        format!("off {}", off.error().unwrap_or_default())
                    }))
                    .child(text(move || {
                        format!("on {}", on.error().unwrap_or_default())
                    }))
            },
        ))
}

#[test]
fn retry_on_mount_false_skips_a_mount_after_an_error() {
    // TanStack shouldLoadOnMount: no data, and not (error with retryOnMount false).
    let mut app = start(Root::of::<RetryOnMountApp>());
    app.wait_for("text:on down").unwrap();
    app.press("text:toggle").unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:on down").unwrap();
    assert_eq!(ROFF.load(Ordering::SeqCst), 1);
    assert_eq!(RON.load(Ordering::SeqCst), 2);
}

static ALWAYS_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn AlwaysApp() -> impl IntoElement {
    let q = use_query_with::<usize, String, _, _>(
        "surface-always",
        QueryOptions::new().network_mode(NetworkMode::Always),
        |_| async { Ok(ALWAYS_LOADS.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    view()
        .child(text(move || format!("data {}", q.data().unwrap_or(0))))
        .child(text(move || format!("fetch {:?}", q.fetch_status())))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("refetch").on_press(move || q.refetch()))
}

#[test]
fn network_mode_always_runs_while_offline() {
    // TanStack networkMode 'always': fetches never pause for the network.
    let mut app = start(Root::of::<AlwaysApp>());
    app.wait_for("text:data 1").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:refetch").unwrap();
    app.wait_for("text:data 2").unwrap();
    assert!(app.shows("fetch Idle"));
}

static FIRST_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn OfflineFirstApp() -> impl IntoElement {
    let go = signal(false);
    let q = use_query_with::<usize, String, _, _>(
        "surface-offline-first",
        QueryOptions::new()
            .network_mode(NetworkMode::OfflineFirst)
            .retry(RetryPolicy::new(1).with_delay(100))
            .enabled(move || go.get()),
        |_| async {
            FIRST_LOADS.fetch_add(1, Ordering::SeqCst);
            Err::<usize, String>("boom".to_owned())
        },
    );
    view()
        .child(text(move || format!("fetch {:?}", q.fetch_status())))
        .child(text(move || {
            format!("error {}", q.error().unwrap_or_default())
        }))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("online").on_press(|| online_manager().set_online(true)))
        .child(text("go").on_press(move || go.set(true)))
}

#[test]
fn network_mode_offline_first_runs_the_first_attempt_then_pauses_retries() {
    // TanStack networkMode 'offlineFirst': the first attempt runs, retries wait for the network.
    let mut app = start(Root::of::<OfflineFirstApp>());
    app.wait_for("text:fetch Idle").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:go").unwrap();
    app.wait_for("text:fetch Paused").unwrap();
    assert_eq!(FIRST_LOADS.load(Ordering::SeqCst), 1);
    app.run_for(ms(300)).unwrap();
    assert_eq!(FIRST_LOADS.load(Ordering::SeqCst), 1);
    app.press("text:online").unwrap();
    app.wait_for("text:error boom").unwrap();
    assert_eq!(FIRST_LOADS.load(Ordering::SeqCst), 2);
}

#[component]
pub fn SelectApp() -> impl IntoElement {
    let q =
        use_query_with::<String, String, _, _>("surface-select", QueryOptions::new(), |_| async {
            Ok("hello".to_owned())
        });
    let len = q.select(|s: &String| s.len());
    text(move || format!("len {:?}", len.get()))
}

#[test]
fn select_maps_the_data() {
    // TanStack: select transforms data; with no data it is not called.
    let mut app = start(Root::of::<SelectApp>());
    app.wait_for("text:len Some(5)").unwrap();
}

// ---- QueryClient reads and defaults -------------------------------------

#[component]
pub fn ReadApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<String, String>("surface-read", "stored".to_owned());
    let data = client.get_query_data::<String, String>("surface-read");
    let state = client.get_query_state("surface-read");
    let missing = client.get_query_state("surface-missing").is_none();
    let shown = format!(
        "data {} state {:?} {} missing {}",
        data.unwrap_or_default(),
        state.as_ref().map(|s| s.status),
        state.map(|s| s.data_update_count).unwrap_or(0),
        missing
    );
    text(shown)
}

#[test]
fn get_query_data_and_get_query_state_read_the_cache() {
    // TanStack: getQueryData returns the value; getQueryState reports status and update counts.
    let mut app = start(Root::of::<ReadApp>());
    app.wait_for("text:data stored state Some(Success) 1 missing true")
        .unwrap();
}

#[component]
pub fn DefaultsApp() -> impl IntoElement {
    query_client().set_default_options(QueryOptions::new().gc(seconds(10)));
    let gc = query_client().get_default_options().gc.as_secs();
    let loads = signal(0_usize);
    let shown = signal(true);
    let counted = move || {
        use_query_with::<usize, String, _, _>(
            "surface-defaults",
            QueryOptions::new(),
            move |_| async move {
                loads.update(|n| *n += 1);
                Ok(1)
            },
        )
    };
    view()
        .child(text(move || format!("gc {gc}")))
        .child(text(move || format!("loads {}", loads.get())))
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(
            move || shown.get(),
            move || {
                let q = counted();
                text(move || format!("q {}", q.data().unwrap_or(0)))
            },
        ))
}

#[test]
fn get_default_options_and_set_default_options_apply_to_new_queries() {
    // TanStack setDefaultOptions: queries created afterwards inherit them, here gcTime 10 s.
    let mut app = start(Root::of::<DefaultsApp>());
    app.wait_for("text:gc 10").unwrap();
    app.wait_for("text:loads 1").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(seconds(11)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:loads 2").unwrap();
}

#[component]
pub fn QueryDefaultsReadApp() -> impl IntoElement {
    let client = query_client();
    client.set_query_defaults(
        QueryKey::from(["surface-qd"]),
        QueryOptions::new().gc(seconds(7)),
    );
    let found = client
        .get_query_defaults(QueryKey::from(["surface-qd"]))
        .map(|o| o.gc.as_secs());
    let missing = client
        .get_query_defaults(QueryKey::from(["surface-other"]))
        .is_none();
    text(format!("qd {found:?} {missing}"))
}

#[test]
fn get_query_defaults_reads_back_a_prefix_default() {
    // TanStack getQueryDefaults returns what setQueryDefaults stored for that key.
    let mut app = start(Root::of::<QueryDefaultsReadApp>());
    app.wait_for("text:qd Some(7) true").unwrap();
}

#[component]
pub fn MutationDefaultsApp() -> impl IntoElement {
    let client = query_client();
    client.set_mutation_defaults(
        QueryKey::from(["surface-md"]),
        MutationDefaults {
            retry: Some(RetryPolicy::new(1).with_delay(10)),
            network_mode: None,
            gc_time: None,
        },
    );
    let stored = client
        .get_mutation_defaults(QueryKey::from(["surface-md"]))
        .is_some();
    let calls = signal(0_u32);
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key(QueryKey::from(["surface-md", "save"])),
        move |v, _| async move {
            let attempt = calls.get_untracked() + 1;
            calls.set(attempt);
            if attempt == 1 {
                Err("first".to_owned())
            } else {
                Ok(v)
            }
        },
    );
    view()
        .child(text(move || format!("md {stored}")))
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text("save").on_press(move || save.mutate(7)))
}

#[test]
fn get_mutation_defaults_and_set_mutation_defaults_apply_retry() {
    // TanStack setMutationDefaults: a mutation whose key matches inherits the retry.
    let mut app = start(Root::of::<MutationDefaultsApp>());
    app.wait_for("text:md true").unwrap();
    app.press("text:save").unwrap();
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:saved 7").unwrap();
}

#[component]
pub fn CancelApp() -> impl IntoElement {
    let n = signal(0_u32);
    let q = use_query_with::<u32, String, _, _>(
        "surface-cancel",
        QueryOptions::new(),
        move |_| async move {
            let run = n.get_untracked() + 1;
            n.set(run);
            timer(ms(100)).await;
            Ok(run)
        },
    );
    view()
        .child(text(move || format!("data {}", q.data().unwrap_or(0))))
        .child(text(move || format!("fe {}", q.is_fetching())))
        .child(text("refetch").on_press(move || q.refetch()))
        .child(
            text("cancel")
                .on_press(|| query_client().cancel_queries(QueryKey::from(["surface-cancel"]))),
        )
}

#[test]
fn cancel_queries_keeps_data_and_drops_the_late_answer() {
    // TanStack cancelQueries (revert): fetchStatus goes idle, data stays, the late answer is ignored.
    let mut app = start(Root::of::<CancelApp>());
    app.run_for(ms(100)).unwrap();
    app.wait_for("text:data 1").unwrap();
    app.press("text:refetch").unwrap();
    app.wait_for("text:fe true").unwrap();
    app.press("text:cancel").unwrap();
    app.wait_for("text:fe false").unwrap();
    app.run_for(ms(200)).unwrap();
    assert!(app.shows("data 1"));
}

static PAUSE_SAVES: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ResumeApp() -> impl IntoElement {
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |v, _| async move {
            PAUSE_SAVES.fetch_add(1, Ordering::SeqCst);
            Ok(v)
        });
    let q = use_query_with::<usize, String, _, _>(
        "surface-resume-query",
        QueryOptions::new(),
        |_| async { Ok(1) },
    );
    view()
        .child(text(move || format!("paused {}", save.is_paused())))
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text(move || format!("fetch {:?}", q.fetch_status())))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("save").on_press(move || save.mutate(3)))
        .child(text("refetch").on_press(move || q.refetch()))
        .child(text("resume").on_press(|| query_client().resume_paused_mutations()))
}

#[test]
fn resume_paused_mutations_runs_mutations_but_not_queries() {
    // TanStack resumePausedMutations continues paused mutations even offline; queries stay paused.
    let mut app = start(Root::of::<ResumeApp>());
    app.wait_for("text:fetch Idle").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:paused true").unwrap();
    app.press("text:refetch").unwrap();
    app.wait_for("text:fetch Paused").unwrap();
    app.press("text:resume").unwrap();
    app.wait_for("text:saved 3").unwrap();
    assert_eq!(PAUSE_SAVES.load(Ordering::SeqCst), 1);
    assert!(app.shows("fetch Paused"));
}

// ---- Infinite queries ----------------------------------------------------

#[derive(Clone)]
struct Doc {
    n: u32,
}

static FLAG_LOADS: AtomicUsize = AtomicUsize::new(0);

async fn slow_doc(n: u32) -> Result<Doc, String> {
    FLAG_LOADS.fetch_add(1, Ordering::SeqCst);
    timer(ms(50)).await;
    Ok(Doc { n })
}

#[component]
pub fn FlagsInfiniteApp() -> impl IntoElement {
    let pages = use_infinite_query::<u32, Doc, String, _, _>(
        "surface-flags",
        InfiniteQueryOptions::new(2_u32, |d: &Doc| (d.n < 3).then_some(d.n + 1))
            .get_previous(|d: &Doc| (d.n > 1).then_some(d.n - 1))
            .retry(RetryPolicy::no_retries()),
        |n, _| slow_doc(n),
    );
    view()
        .child(text(move || {
            format!(
                "flags {} {}",
                pages.is_fetching_next_page(),
                pages.is_fetching_previous_page()
            )
        }))
        .child(text(move || format!("params {:?}", pages.page_params())))
        .child(text("next").on_press(move || pages.fetch_next_page()))
        .child(text("previous").on_press(move || pages.fetch_previous_page()))
}

#[test]
fn infinite_fetching_flags_and_page_params_follow_tanstack() {
    // TanStack: isFetchingNextPage and isFetchingPreviousPage per direction; pageParams in page order.
    let mut app = start(Root::of::<FlagsInfiniteApp>());
    app.run_for(ms(50)).unwrap();
    app.wait_for("text:params [2]").unwrap();
    app.press("text:next").unwrap();
    app.wait_for("text:flags true false").unwrap();
    app.run_for(ms(50)).unwrap();
    app.wait_for("text:params [2, 3]").unwrap();
    app.press("text:previous").unwrap();
    app.wait_for("text:flags false true").unwrap();
    app.run_for(ms(50)).unwrap();
    app.wait_for("text:params [1, 2, 3]").unwrap();
}

static NEXT_FAILS: AtomicU32 = AtomicU32::new(0);

async fn failing_next(n: u32) -> Result<Doc, String> {
    timer(ms(10)).await;
    if n == 3 && NEXT_FAILS.fetch_add(1, Ordering::SeqCst) == 0 {
        return Err("no page 3".to_owned());
    }
    Ok(Doc { n })
}

#[component]
pub fn NextErrorApp() -> impl IntoElement {
    let pages = use_infinite_query::<u32, Doc, String, _, _>(
        "surface-next-error",
        InfiniteQueryOptions::new(1_u32, |d: &Doc| (d.n < 3).then_some(d.n + 1))
            .get_previous(|_: &Doc| None)
            .retry(RetryPolicy::no_retries()),
        |n, _| failing_next(n),
    );
    view()
        .child(text(move || {
            format!(
                "errs {} {}",
                pages.is_fetch_next_page_error(),
                pages.is_fetch_previous_page_error()
            )
        }))
        .child(text(move || format!("count {}", pages.page_count())))
        .child(text("next").on_press(move || pages.fetch_next_page()))
}

#[test]
fn fetch_next_page_error_flag_is_set_for_next_only() {
    // TanStack isFetchNextPageError: error while the direction is forward.
    let mut app = start(Root::of::<NextErrorApp>());
    app.wait_for("text:count 1").unwrap();
    app.press("text:next").unwrap();
    app.run_for(ms(10)).unwrap();
    app.press("text:next").unwrap();
    app.wait_for("text:errs true false").unwrap();
    assert!(app.shows("count 2"));
}

async fn failing_previous(n: u32) -> Result<Doc, String> {
    timer(ms(10)).await;
    if n == 1 {
        return Err("no page 1".to_owned());
    }
    Ok(Doc { n })
}

#[component]
pub fn PreviousErrorApp() -> impl IntoElement {
    let pages = use_infinite_query::<u32, Doc, String, _, _>(
        "surface-prev-error",
        InfiniteQueryOptions::new(2_u32, |d: &Doc| (d.n < 3).then_some(d.n + 1))
            .get_previous(|d: &Doc| (d.n > 1).then_some(d.n - 1))
            .retry(RetryPolicy::no_retries()),
        |n, _| failing_previous(n),
    );
    view()
        .child(text(move || {
            format!(
                "errs {} {}",
                pages.is_fetch_next_page_error(),
                pages.is_fetch_previous_page_error()
            )
        }))
        .child(text(move || format!("count {}", pages.page_count())))
        .child(text("previous").on_press(move || pages.fetch_previous_page()))
}

#[test]
fn fetch_previous_page_error_flag_is_set_for_previous_only() {
    // TanStack isFetchPreviousPageError: error while the direction is backward.
    let mut app = start(Root::of::<PreviousErrorApp>());
    app.wait_for("text:count 1").unwrap();
    app.press("text:previous").unwrap();
    app.wait_for("text:errs false true").unwrap();
}

static ENSURE_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn EnsureInfiniteApp() -> impl IntoElement {
    let result = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", result.get())))
        .child(text("ensure").on_press(move || {
            spawn(async move {
                let out = client
                    .ensure_infinite_query_data(
                        "surface-ensure-inf",
                        InfiniteQueryOptions::new(1_u32, |_: &Doc| None)
                            .retry(RetryPolicy::no_retries()),
                        |n, _| async move {
                            ENSURE_LOADS.fetch_add(1, Ordering::SeqCst);
                            Ok::<Doc, String>(Doc { n })
                        },
                    )
                    .await;
                result.set(match out {
                    Ok(pages) => format!("pages {}", pages.len()),
                    Err(_) => "failed".to_owned(),
                });
            });
        }))
}

#[test]
fn ensure_infinite_query_data_returns_cache_then_loads() {
    // TanStack ensureInfiniteQueryData: cached pages are returned without a new load.
    let mut app = start(Root::of::<EnsureInfiniteApp>());
    app.wait_for("text:result none").unwrap();
    app.press("text:ensure").unwrap();
    app.wait_for("text:result pages 1").unwrap();
    app.press("text:ensure").unwrap();
    app.run_for(ms(10)).unwrap();
    assert_eq!(ENSURE_LOADS.load(Ordering::SeqCst), 1);
}

static VERSION: AtomicU32 = AtomicU32::new(1);

#[component]
pub fn FetchInfiniteApp() -> impl IntoElement {
    let pages = use_infinite_query::<u32, String, String, _, _>(
        "surface-fetch-inf",
        InfiniteQueryOptions::new(1_u32, |_: &String| None).retry(RetryPolicy::no_retries()),
        |n, _| async move {
            timer(ms(10)).await;
            Ok::<String, String>(format!("v{}-{n}", VERSION.load(Ordering::SeqCst)))
        },
    );
    let client = query_client();
    let result = signal(String::from("none"));
    view()
        .child(text(move || {
            format!(
                "shown {}",
                pages
                    .pages()
                    .iter()
                    .map(|p| p.as_str().to_owned())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }))
        .child(text(move || format!("result {}", result.get())))
        .child(text("bump").on_press(|| {
            VERSION.fetch_add(1, Ordering::SeqCst);
        }))
        .child(text("fetch").on_press(move || {
            spawn(async move {
                let out = client
                    .fetch_infinite_query(
                        "surface-fetch-inf",
                        InfiniteQueryOptions::new(1_u32, |_: &String| None)
                            .retry(RetryPolicy::no_retries()),
                        |n, _| async move {
                            timer(ms(10)).await;
                            Ok::<String, String>(format!("v{}-{n}", VERSION.load(Ordering::SeqCst)))
                        },
                    )
                    .await;
                result.set(match out {
                    Ok(pages) => pages
                        .iter()
                        .map(|p| p.as_str().to_owned())
                        .collect::<Vec<_>>()
                        .join(","),
                    Err(FetchQueryError::Failed(e)) => format!("failed {e}"),
                    Err(FetchQueryError::Cancelled) => "cancelled".to_owned(),
                });
            });
        }))
}

#[test]
fn fetch_infinite_query_always_reloads_the_pages() {
    // TanStack fetchInfiniteQuery runs fetchQuery with staleTime 0, so cached pages are reloaded.
    let mut app = start(Root::of::<FetchInfiniteApp>());
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:shown v1-1").unwrap();
    app.press("text:bump").unwrap();
    app.press("text:fetch").unwrap();
    app.run_for(ms(20)).unwrap();
    app.wait_for("text:result v2-1").unwrap();
}

static PREFETCH_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn PrefetchInfiniteApp() -> impl IntoElement {
    query_client().prefetch_infinite_query(
        "surface-prefetch-inf",
        InfiniteQueryOptions::new(1_u32, |_: &Doc| None).retry(RetryPolicy::no_retries()),
        |n, _| async move {
            PREFETCH_LOADS.fetch_add(1, Ordering::SeqCst);
            timer(ms(10)).await;
            Ok::<Doc, String>(Doc { n })
        },
    );
    let pages = use_infinite_query::<u32, Doc, String, _, _>(
        "surface-prefetch-inf",
        InfiniteQueryOptions::new(1_u32, |_: &Doc| None).retry(RetryPolicy::no_retries()),
        |n, _| async move {
            PREFETCH_LOADS.fetch_add(1, Ordering::SeqCst);
            Ok::<Doc, String>(Doc { n })
        },
    );
    text(move || format!("prefetched {}", pages.page_count()))
}

#[test]
fn prefetch_infinite_query_loads_once_for_a_later_observer() {
    // TanStack prefetchInfiniteQuery: the cached pages serve the observer that follows.
    let mut app = start(Root::of::<PrefetchInfiniteApp>());
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:prefetched 1").unwrap();
    assert_eq!(PREFETCH_LOADS.load(Ordering::SeqCst), 1);
}

// ---- Mutations ------------------------------------------------------------

#[component]
pub fn MutationStateApp() -> impl IntoElement {
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |v, _| async move {
            Ok(v + 1)
        });
    view()
        .child(text(move || {
            format!(
                "ok {} {} {} {}",
                save.data().unwrap_or(0),
                save.is_success(),
                save.is_pending(),
                save.is_idle()
            )
        }))
        .child(text("save").on_press(move || save.mutate(7)))
}

#[test]
fn mutation_data_is_success_and_is_pending_follow_the_call() {
    // TanStack: a success exposes data and status success; isPending is status loading.
    let mut app = start(Root::of::<MutationStateApp>());
    app.wait_for("text:ok 0 false false true").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:ok 8 true false false").unwrap();
}

#[component]
pub fn MutateAsyncApp() -> impl IntoElement {
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |v, _| async move {
            if v == 2 {
                Err("nope".to_owned())
            } else {
                Ok(v)
            }
        });
    let result = signal(String::from("none"));
    view()
        .child(text(move || format!("res {}", result.get())))
        .child(text("ok").on_press(move || {
            spawn(async move {
                let out = save.mutate_async(1).await;
                result.set(format!("{out:?}"));
            });
        }))
        .child(text("bad").on_press(move || {
            spawn(async move {
                let out = save.mutate_async(2).await;
                result.set(format!("{out:?}"));
            });
        }))
}

#[test]
fn mutate_async_resolves_with_the_result_or_the_error() {
    // TanStack mutateAsync resolves with data or rejects with the error. Here the value is
    // Some(Ok/Err), and None means a newer call superseded it.
    let mut app = start(Root::of::<MutateAsyncApp>());
    app.wait_for("text:res none").unwrap();
    app.press("text:ok").unwrap();
    app.wait_for("text:res Some(Ok(1))").unwrap();
    app.press("text:bad").unwrap();
    app.wait_for("text:res Some(Err(\"nope\"))").unwrap();
}

#[component]
pub fn MutationResetApp() -> impl IntoElement {
    let save = use_mutation_with_context::<u32, u32, String, String, _, _>(
        MutationOptions::new().on_mutate(|v: &u32| format!("ctx {v}")),
        |v, _| async move { Ok(v) },
    );
    view()
        .child(text(move || {
            format!(
                "reset {} {} {} {}",
                save.is_idle(),
                save.data().is_none(),
                save.variables().is_none(),
                save.context().is_none()
            )
        }))
        .child(text("save").on_press(move || save.mutate(4)))
        .child(text("reset").on_press(move || save.reset()))
}

#[test]
fn mutation_reset_returns_to_idle() {
    // TanStack reset: status idle, data, variables and context cleared.
    let mut app = start(Root::of::<MutationResetApp>());
    app.wait_for("text:reset true true true true").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:reset false false false false").unwrap();
    app.press("text:reset").unwrap();
    app.wait_for("text:reset true true true true").unwrap();
}

static RETRY_DELAY_ARGS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static RETRY_CALLS: AtomicU32 = AtomicU32::new(0);

#[component]
pub fn MutationRetryDelayApp() -> impl IntoElement {
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new()
            .retry(RetryPolicy::new(1))
            .retry_delay(|failures, _: &String| {
                RETRY_DELAY_ARGS.lock().unwrap().push(failures);
                ms(100)
            }),
        |v, _| async move {
            if RETRY_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
                Err("again".to_owned())
            } else {
                Ok(v)
            }
        },
    );
    view()
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text("save").on_press(move || save.mutate(5)))
}

#[test]
fn mutation_retry_delay_gets_the_failures_before_this_one() {
    // TanStack retryDelay receives the failure count before the current failure: 0 here.
    let mut app = start(Root::of::<MutationRetryDelayApp>());
    app.wait_for("text:saved 0").unwrap();
    app.press("text:save").unwrap();
    app.run_for(ms(99)).unwrap();
    assert_eq!(RETRY_CALLS.load(Ordering::SeqCst), 1);
    app.run_for(ms(1)).unwrap();
    app.wait_for("text:saved 5").unwrap();
    assert_eq!(*RETRY_DELAY_ARGS.lock().unwrap(), vec![0]);
}

static ALWAYS_SAVES: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn MutationAlwaysApp() -> impl IntoElement {
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().network_mode(NetworkMode::Always),
        |v, _| async move {
            ALWAYS_SAVES.fetch_add(1, Ordering::SeqCst);
            Ok(v)
        },
    );
    view()
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("save").on_press(move || save.mutate(9)))
}

#[test]
fn mutation_network_mode_always_runs_while_offline() {
    // TanStack networkMode 'always' on a mutation: no pause before the mutator.
    let mut app = start(Root::of::<MutationAlwaysApp>());
    app.wait_for("text:saved 0").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:saved 9").unwrap();
    assert_eq!(ALWAYS_SAVES.load(Ordering::SeqCst), 1);
}

static FIRST_CALLS: AtomicU32 = AtomicU32::new(0);

#[component]
pub fn MutationOfflineFirstApp() -> impl IntoElement {
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new()
            .network_mode(NetworkMode::OfflineFirst)
            .retry(RetryPolicy::new(1).with_delay(50)),
        |v, _| async move {
            if FIRST_CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
                Err("again".to_owned())
            } else {
                Ok(v)
            }
        },
    );
    view()
        .child(text(move || format!("paused {}", save.is_paused())))
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("online").on_press(|| online_manager().set_online(true)))
        .child(text("save").on_press(move || save.mutate(6)))
}

#[test]
fn mutation_network_mode_offline_first_runs_once_then_waits_for_retries() {
    // TanStack networkMode 'offlineFirst': the first attempt runs; a retry waits for the network.
    let mut app = start(Root::of::<MutationOfflineFirstApp>());
    app.wait_for("text:paused false").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:paused true").unwrap();
    assert_eq!(FIRST_CALLS.load(Ordering::SeqCst), 1);
    app.press("text:online").unwrap();
    app.wait_for("text:saved 6").unwrap();
    assert_eq!(FIRST_CALLS.load(Ordering::SeqCst), 2);
}

#[component]
pub fn MutationSuccessCtxApp() -> impl IntoElement {
    let log = signal(String::new());
    let save = use_mutation_with_context::<u32, u32, String, String, _, _>(
        MutationOptions::new()
            .on_mutate(|_: &u32| String::from("c1"))
            .on_success_ctx(move |_: &u32, _: &u32, c: &String| {
                log.update(|s| s.push_str(&format!("ok:{c};")));
            }),
        |v, _| async move { Ok(v) },
    );
    view()
        .child(text(move || format!("log {}", log.get())))
        .child(text("save").on_press(move || save.mutate(1)))
}

#[test]
fn mutation_on_success_ctx_receives_the_context() {
    // TanStack onSuccess(data, variables, context).
    let mut app = start(Root::of::<MutationSuccessCtxApp>());
    app.wait_for("text:save").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:log ok:c1;").unwrap();
}
