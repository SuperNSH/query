//! Tests for the features finished in the second round. Each one goes through a real UI
//! on the manual clock, and each checks the TanStack behavior named in its comment.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    InfiniteQueryOptions, MutateOptions, MutationOptions, QueryConfig, QueryFilters, QueryKey,
    QueryOptions, hash_key, match_query, query_client, use_mutation_state, use_mutation_with,
    use_prefetch_query, use_queries, use_query_config, use_query_with,
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

static LEFT_RUNS: AtomicUsize = AtomicUsize::new(0);
static RIGHT_RUNS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn PerIndexApp() -> impl IntoElement {
    let queries = use_queries(
        vec![
            ("left", QueryOptions::new()),
            ("right", QueryOptions::new()),
        ],
        |context| async move {
            // The right query loads later, so the left binding must not wake for it.
            let slow = context.key.parts()[0].as_ref() == "right";
            timer(ms(if slow { 30 } else { 5 })).await;
            Ok::<String, String>(format!("{} loaded", context.key.parts()[0]))
        },
    );
    let (left, right) = (queries[0], queries[1]);
    view()
        .child(text(move || {
            LEFT_RUNS.fetch_add(1, Ordering::SeqCst);
            format!("left {}", left.data().unwrap_or_default())
        }))
        .child(text(move || {
            RIGHT_RUNS.fetch_add(1, Ordering::SeqCst);
            format!("right {}", right.data().unwrap_or_default())
        }))
}

#[test]
fn use_queries_results_are_fine_grained_per_index() {
    // Each entry of use_queries is its own query, so a binding on one index does not
    // wake when another index changes.
    let mut app = start(Root::of::<PerIndexApp>());
    app.wait_for("text:left left loaded").unwrap();
    let left_runs = LEFT_RUNS.load(Ordering::SeqCst);
    app.run_for(ms(30)).unwrap();
    app.wait_for("text:right right loaded").unwrap();
    assert_eq!(LEFT_RUNS.load(Ordering::SeqCst), left_runs);
}

#[component]
pub fn StructuralApp() -> impl IntoElement {
    let same = use_query_config::<usize, String, _, _>(
        "structural-same",
        QueryConfig::<usize, String>::from(QueryOptions::new())
            .structural_sharing(|a: &usize, b: &usize| a == b),
        |_| async { Ok(5) },
    );
    let varied = use_query_with::<usize, String, _, _>(
        "structural-varied",
        QueryOptions::new(),
        |_| async {
            static N: AtomicUsize = AtomicUsize::new(0);
            Ok(N.fetch_add(1, Ordering::SeqCst) + 1)
        },
    );
    view()
        .child(text(move || {
            SAME_RUNS.fetch_add(1, Ordering::SeqCst);
            format!("same {}", same.data().unwrap_or(0))
        }))
        .child(text(move || {
            VARIED_RUNS.fetch_add(1, Ordering::SeqCst);
            format!("varied {}", varied.data().unwrap_or(0))
        }))
        .child(text("refetch").on_press(move || {
            same.refetch();
            varied.refetch();
        }))
}

static SAME_RUNS: AtomicUsize = AtomicUsize::new(0);
static VARIED_RUNS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn structural_sharing_keeps_a_binding_quiet_for_an_equal_answer() {
    // TanStack structuralSharing: an equal answer keeps the old data, so a binding on it
    // does not run again. A different answer still updates.
    let mut app = start(Root::of::<StructuralApp>());
    app.wait_for("text:same 5").unwrap();
    app.wait_for("text:varied 1").unwrap();
    let (same, varied) = (
        SAME_RUNS.load(Ordering::SeqCst),
        VARIED_RUNS.load(Ordering::SeqCst),
    );
    app.press("text:refetch").unwrap();
    app.wait_for("text:varied 2").unwrap();
    assert_eq!(
        SAME_RUNS.load(Ordering::SeqCst),
        same,
        "equal answer must not wake"
    );
    assert_eq!(VARIED_RUNS.load(Ordering::SeqCst), varied + 1);
}

static SKIP_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn SkipApp() -> impl IntoElement {
    // TanStack skipToken: the query never fetches. Here that is enabled(false).
    let q = use_query_with::<usize, String, _, _>(
        "skipped",
        QueryOptions::new().enabled(|| false),
        |_| async {
            SKIP_LOADS.fetch_add(1, Ordering::SeqCst);
            Ok(1)
        },
    );
    text(move || format!("skip {:?}", q.data()))
}

#[test]
fn skip_token_maps_to_enabled_false() {
    let mut app = start(Root::of::<SkipApp>());
    app.wait_for("text:skip None").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert_eq!(SKIP_LOADS.load(Ordering::SeqCst), 0);
    assert!(app.shows("skip None"));
}

static GC_TOGGLE: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn GcMutationApp() -> impl IntoElement {
    let shown = signal(true);
    let states = use_mutation_state(QueryKey::from("gc-log"), |s| format!("{:?}", s.status));
    view()
        .child(text(move || format!("listed {}", states.get().len())))
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(
            move || shown.get(),
            || {
                let save = use_mutation_with::<u32, u32, String, _, _>(
                    MutationOptions::new()
                        .mutation_key(QueryKey::from("gc-log"))
                        .gc_time(seconds(2)),
                    |v, _| async move {
                        timer(ms(10)).await;
                        Ok(v)
                    },
                );
                if GC_TOGGLE.fetch_add(1, Ordering::SeqCst) == 0 {
                    save.mutate(1);
                }
                text(move || format!("saved {:?}", save.data()))
            },
        ))
}

#[test]
fn mutation_gc_time_keeps_a_settled_call_listed_then_removes_it() {
    // TanStack mutation gcTime: a settled mutation with no observer leaves the cache after it.
    let mut app = start(Root::of::<GcMutationApp>());
    app.wait_for("text:listed 1").unwrap();
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:saved Some(1)").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert!(app.shows("listed 1"));
    app.run_for(seconds(2)).unwrap();
    app.wait_for("text:listed 0").unwrap();
}

static SLOW_RESULT: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn MutateAsyncApp() -> impl IntoElement {
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |v, _| async move {
            if v == 1 {
                timer(ms(100)).await;
            }
            Ok(v)
        });
    let first = signal(String::from("none"));
    let second = signal(String::from("none"));
    view()
        .child(text(move || format!("data {}", save.data().unwrap_or(0))))
        .child(text(move || format!("first {}", first.get())))
        .child(text(move || format!("second {}", second.get())))
        .child(text("slow").on_press(move || {
            spawn(async move {
                let out = save.mutate_async(1).await;
                first.set(format!("{out:?}"));
            });
        }))
        .child(text("fast").on_press(move || {
            spawn(async move {
                let out = save.mutate_async(2).await;
                second.set(format!("{out:?}"));
            });
        }))
}

#[test]
fn mutate_async_replaced_call_resolves_with_its_own_result() {
    // TanStack mutateAsync: a call replaced by a newer one still resolves with its own result.
    let mut app = start(Root::of::<MutateAsyncApp>());
    app.wait_for("text:data 0").unwrap();
    app.press("text:slow").unwrap();
    app.press("text:fast").unwrap();
    app.wait_for("text:second Some(Ok(2))").unwrap();
    app.run_for(ms(100)).unwrap();
    app.wait_for("text:first Some(Ok(1))").unwrap();
    assert!(app.shows("data 2"), "the latest call owns the state");
    let _ = SLOW_RESULT.load(Ordering::SeqCst);
}

#[component]
pub fn MutateWithApp() -> impl IntoElement {
    let log = signal(String::new());
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |v, _| async move {
            if v == 1 {
                timer(ms(100)).await;
            }
            Ok(v)
        });
    view()
        .child(text(move || format!("log {}", log.get())))
        .child(text("a").on_press(move || {
            save.mutate_with(
                1,
                MutateOptions::new().on_success(move |_: &u32, _: &u32| {
                    log.update(|s| s.push_str("a;"));
                }),
            );
        }))
        .child(text("b").on_press(move || {
            save.mutate_with(
                2,
                MutateOptions::new().on_success(move |_: &u32, _: &u32| {
                    log.update(|s| s.push_str("b;"));
                }),
            );
        }))
}

#[test]
fn mutate_with_callbacks_run_for_the_latest_call_only() {
    // TanStack mutate options: the callback runs for the call it was given. Here the first call
    // is replaced, so only the second call's callback runs.
    let mut app = start(Root::of::<MutateWithApp>());
    app.wait_for("text:log ").unwrap();
    app.press("text:a").unwrap();
    app.press("text:b").unwrap();
    app.run_for(ms(150)).unwrap();
    app.wait_for("text:log b;").unwrap();
    assert!(app.shows("log b;"));
}

static PREFETCH_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn PrefetchApp() -> impl IntoElement {
    use_prefetch_query(["prefetch-me"], QueryOptions::new(), |_| async {
        PREFETCH_LOADS.fetch_add(1, Ordering::SeqCst);
        Ok::<String, String>("warm".to_owned())
    });
    let later =
        use_query_with::<String, String, _, _>(["prefetch-me"], QueryOptions::new(), |_| async {
            PREFETCH_LOADS.fetch_add(1, Ordering::SeqCst);
            Ok("warm".to_owned())
        });
    text(move || format!("later {}", later.data().unwrap_or_default()))
}

#[test]
fn use_prefetch_query_loads_for_a_later_observer() {
    // TanStack usePrefetchQuery: the prefetched value serves the observer that follows, with
    // no second load. Both sides use the same loader, as an app does for one key.
    let mut app = start(Root::of::<PrefetchApp>());
    app.wait_for("text:later warm").unwrap();
    assert_eq!(PREFETCH_LOADS.load(Ordering::SeqCst), 1);
}

static UNSUB_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn UnsubscribedApp() -> impl IntoElement {
    let q = use_query_with::<String, String, _, _>(
        "unsubscribed",
        QueryOptions::new().subscribed(false),
        |_| async {
            UNSUB_LOADS.fetch_add(1, Ordering::SeqCst);
            Ok("fetched".to_owned())
        },
    );
    text(move || format!("unsub {}", q.data().unwrap_or_else(|| "none".to_owned())))
}

#[test]
fn subscribed_false_reads_the_cache_and_never_fetches() {
    // TanStack subscribed: false means the observer does not fetch or follow the entry.
    let mut app = start(Root::of::<UnsubscribedApp>());
    app.wait_for("text:unsub none").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert_eq!(UNSUB_LOADS.load(Ordering::SeqCst), 0);
}

#[component]
pub fn MatchApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<String, String>(["users", "1"], "ann".to_owned());
    let state = client.get_query_state(["users", "1"]).unwrap();
    let by_prefix = match_query(&QueryFilters::from("users"), &state);
    let by_other = match_query(&QueryFilters::from("posts"), &state);
    text(format!(
        "match {by_prefix} {by_other} hash {}",
        hash_key(&QueryKey::from(["a", "b"]))
    ))
}

#[test]
fn match_query_and_hash_key_follow_tanstack() {
    // TanStack matchQuery: a prefix matches. hashKey: a stable JSON array of the parts.
    let mut app = start(Root::of::<MatchApp>());
    app.wait_for("text:match true false hash [\"a\",\"b\"]")
        .unwrap();
}

static INF_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn InfiniteQueryApp() -> impl IntoElement {
    let result = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", result.get())))
        .child(text("run").on_press(move || {
            spawn(async move {
                let out = client
                    .infinite_query(
                        "infinite-alias",
                        InfiniteQueryOptions::new(1_u32, |_: &u32| None),
                        |n, _| async move {
                            INF_LOADS.fetch_add(1, Ordering::SeqCst);
                            Ok::<u32, String>(n)
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
fn client_infinite_query_reloads_like_fetch_infinite_query() {
    // TanStack queryClient.infiniteQuery: the same as fetchInfiniteQuery, which reloads.
    let mut app = start(Root::of::<InfiniteQueryApp>());
    app.wait_for("text:result none").unwrap();
    app.press("text:run").unwrap();
    app.wait_for("text:result pages 1").unwrap();
    app.press("text:run").unwrap();
    app.run_for(ms(10)).unwrap();
    assert_eq!(INF_LOADS.load(Ordering::SeqCst), 2);
}
