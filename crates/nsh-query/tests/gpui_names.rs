//! Tests for the TanStack and gpui-query names added in the finish round. Each goes through a
//! real UI on the manual clock, and each counts what it checks, so a broken behavior fails.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    CachePolicy, Fetched, InfiniteQueryOptions, MutationCacheEvent, MutationFilters,
    MutationOptions, MutationStatus, QueryOptions, QueryStatus, RequestPolicy, query_client,
    use_diagnostics, use_infinite_query, use_mutation_with, use_query_manual,
    use_query_manual_opts, use_query_select, use_query_unsignalled, use_query_unsignalled_opts,
    use_query_with, use_query_with_policy,
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

fn count(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::SeqCst)
}

static MUTATION_ADDED: AtomicUsize = AtomicUsize::new(0);
static MUTATION_REMOVED: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn MutationCacheApp() -> impl IntoElement {
    let subscription = Rc::new(RefCell::new(Some(
        query_client()
            .get_mutation_cache()
            .subscribe(|event| match event {
                MutationCacheEvent::Added(_) => {
                    MUTATION_ADDED.fetch_add(1, Ordering::SeqCst);
                }
                MutationCacheEvent::Removed(_) => {
                    MUTATION_REMOVED.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }),
    )));
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |v, _| async move {
            Ok(v + 1)
        });
    let second = signal(false);
    let stop = subscription.clone();
    view()
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text("save").on_press(move || save.mutate(1)))
        .child(text("remove").on_press(move || {
            query_client()
                .get_mutation_cache()
                .remove(MutationFilters::default());
        }))
        .child(text("stop").on_press(move || {
            if let Some(subscription) = stop.borrow_mut().take() {
                subscription.stop();
            }
        }))
        .child(text("toggle").on_press(move || second.update(|s| *s = !*s)))
        .child(when(
            move || second.get(),
            || {
                let _again = use_mutation_with::<u32, u32, String, _, _>(
                    MutationOptions::new(),
                    |v, _| async move { Ok(v) },
                );
                text("second")
            },
        ))
}

#[test]
fn mutation_cache_subscribe_stops_and_remove_drops_settled_calls() {
    // TanStack mutationCache: subscribe hears adds and removes until its stop; remove drops
    // settled calls. Each step checks the event count it expects.
    let mut app = start(Root::of::<MutationCacheApp>());
    app.wait_for("text:saved 0").unwrap();
    assert_eq!(
        count(&MUTATION_ADDED),
        1,
        "the subscription hears the mount"
    );

    app.press("text:save").unwrap();
    app.wait_for("text:saved 2").unwrap();
    app.press("text:remove").unwrap();
    assert_eq!(
        count(&MUTATION_REMOVED),
        1,
        "remove reports the settled call"
    );

    app.press("text:stop").unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:second").unwrap();
    assert_eq!(
        count(&MUTATION_ADDED),
        1,
        "after stop, a new mutation is not heard"
    );
}

static INF_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn AllInfiniteApp() -> impl IntoElement {
    let pages = use_infinite_query::<u32, u32, String, _, _>(
        "all-inf-a",
        InfiniteQueryOptions::new(1_u32, |n: &u32| (*n < 2).then_some(*n + 1)),
        |n, _| async move {
            INF_LOADS.fetch_add(1, Ordering::SeqCst);
            Ok(n)
        },
    );
    query_client().prefetch_infinite_query::<u32, u32, String, _, _>(
        "all-inf-b",
        InfiniteQueryOptions::new(1_u32, |_: &u32| None),
        |n, _| async move { Ok(n) },
    );
    let _other = use_infinite_query::<String, String, String, _, _>(
        "all-inf-c",
        InfiniteQueryOptions::new(String::from("c"), |_: &String| None),
        |s, _| async move { Ok(s) },
    );
    let _plain = use_query_with::<u32, String, _, _>("all-plain", QueryOptions::new(), |_| async {
        Ok(7_u32)
    });
    view().child(text(move || {
        format!(
            "pages {} listed {} other {}",
            pages.page_count(),
            query_client()
                .all_infinite_queries::<u32, u32, String>()
                .len(),
            query_client()
                .all_infinite_queries::<String, String, String>()
                .len()
        )
    }))
}

#[test]
fn all_infinite_queries_lists_only_the_page_type_asked_for() {
    // gpui-query all_infinite_queries: every infinite query with the same page, value and error
    // types. A plain query and an infinite query of another page type are not listed.
    let mut app = start(Root::of::<AllInfiniteApp>());
    app.run_for(ms(20)).unwrap();
    app.wait_for("text:pages 1 listed 2 other 1").unwrap();
}

#[component]
pub fn DiagnosticsApp() -> impl IntoElement {
    let _query = use_query_with::<u32, String, _, _>(
        "diag-q",
        QueryOptions::new().ttl(seconds(60)),
        |_| async {
            timer(ms(5)).await;
            Ok(5_u32)
        },
    );
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key(["diag-m"]),
        |v, _| async move { Ok(v) },
    );
    let diagnostics = use_diagnostics();
    view()
        .child(text(move || {
            let d = diagnostics.get();
            format!("queries {} mutations {}", d.query_count, d.mutation_count)
        }))
        .child(text(move || {
            let d = diagnostics.get();
            match d.queries.first() {
                Some(q) => format!(
                    "query {} success {} age {} retries {}",
                    q.key,
                    q.status == QueryStatus::Success,
                    q.cache_age_ms.is_some(),
                    q.retry_count
                ),
                None => "query none".to_owned(),
            }
        }))
        .child(text(move || {
            let d = diagnostics.get();
            match d.mutations.first() {
                Some(m) => format!(
                    "mutation {} success {}",
                    m.key.clone().unwrap_or_default(),
                    m.status == MutationStatus::Success
                ),
                None => "mutation none".to_owned(),
            }
        }))
        .child(text("call").on_press(move || save.mutate(3)))
}

#[test]
fn diagnostics_report_queries_and_mutations_after_a_load_and_a_call() {
    // gpui-query diagnostics: a screen reads the cache's counts, a query's status and age, and a
    // mutation's key and status, before and after a load and a call.
    let mut app = start(Root::of::<DiagnosticsApp>());
    app.wait_for("text:queries 1 mutations 1").unwrap();
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:query diag-q success true age true retries 0")
        .unwrap();
    app.press("text:call").unwrap();
    app.wait_for("text:mutation diag-m success true").unwrap();
}

static LATEST_LOADS: AtomicUsize = AtomicUsize::new(0);
static IGNORE_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn RequestPolicyApp() -> impl IntoElement {
    // Each fetcher counts its own invocation before it waits, since a replaced load is cancelled
    // mid-wait and never reaches code after the timer.
    let latest = use_query_with::<usize, String, _, _>(
        "policy-latest",
        QueryOptions::new().request(RequestPolicy::LatestWins),
        |_| async {
            let call = LATEST_LOADS.fetch_add(1, Ordering::SeqCst) + 1;
            timer(ms(10)).await;
            Ok(call)
        },
    );
    let ignore = use_query_with::<usize, String, _, _>(
        "policy-ignore",
        QueryOptions::new().request(RequestPolicy::IgnoreWhileLoading),
        |_| async {
            let call = IGNORE_LOADS.fetch_add(1, Ordering::SeqCst) + 1;
            timer(ms(10)).await;
            Ok(call)
        },
    );
    view()
        .child(text(move || {
            format!("latest {}", latest.data().unwrap_or(0))
        }))
        .child(text(move || {
            format!("ignore {}", ignore.data().unwrap_or(0))
        }))
        .child(text("refetch").on_press(move || {
            latest.refetch();
            ignore.refetch();
        }))
}

#[test]
fn request_policy_latest_wins_replaces_a_load_and_ignore_while_loading_keeps_it() {
    // RequestPolicy::LatestWins starts a second load, which cancels the first. The first's
    // fetcher was invoked, but its answer never lands. RequestPolicy::IgnoreWhileLoading ignores
    // a refetch that arrives during a load, so its fetcher is never invoked a second time.
    let mut app = start(Root::of::<RequestPolicyApp>());
    app.wait_for("text:latest 0").unwrap();
    app.wait_for("text:ignore 0").unwrap();
    app.press("text:refetch").unwrap();
    app.run_for(ms(30)).unwrap();
    app.wait_for("text:latest 2").unwrap();
    app.wait_for("text:ignore 1").unwrap();
    assert_eq!(
        count(&LATEST_LOADS),
        2,
        "latest wins: mount and refetch both ran"
    );
    assert_eq!(
        count(&IGNORE_LOADS),
        1,
        "ignore while loading: the refetch was dropped"
    );

    app.run_for(ms(50)).unwrap();
    assert_eq!(
        count(&LATEST_LOADS),
        2,
        "no load starts after the answers arrive"
    );
    assert_eq!(
        count(&IGNORE_LOADS),
        1,
        "no load starts after the answers arrive"
    );
}

#[component]
pub fn PolicyApp() -> impl IntoElement {
    let server = use_query_with_policy::<u32, String, _, _>(
        "policy-server",
        QueryOptions::new(),
        |_| async {
            timer(ms(5)).await;
            Ok(Fetched::with_policy(1_u32, CachePolicy::Ttl { ttl_ms: 0 }))
        },
    );
    let client_side =
        use_query_with::<u32, String, _, _>("policy-client", QueryOptions::new(), |_| async {
            timer(ms(5)).await;
            Ok(2_u32)
        });
    let tick = signal(0_u32);
    spawn(async move {
        loop {
            timer(ms(500)).await;
            tick.update(|n| *n += 1);
        }
    });
    view()
        .child(text(move || {
            tick.get();
            format!(
                "server data {} stale {}",
                server.data().unwrap_or(0),
                server.is_stale()
            )
        }))
        .child(text(move || {
            tick.get();
            format!(
                "client data {} stale {}",
                client_side.data().unwrap_or(0),
                client_side.is_stale()
            )
        }))
}

#[test]
fn use_query_with_policy_sets_the_freshness_the_server_returns() {
    // A Fetched cache policy replaces the default TTL: the server's zero TTL makes the answer
    // stale at once, while the default TTL keeps the client's answer fresh.
    let mut app = start(Root::of::<PolicyApp>());
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:server data 1 stale true").unwrap();
    app.wait_for("text:client data 2 stale false").unwrap();
}

static FETCH_POLICY_SHOWN: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FetchPolicyApp() -> impl IntoElement {
    let shown = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", shown.get())))
        .child(text("run").on_press(move || {
            spawn(async move {
                let out = client
                    .fetch_query_with_policy::<u32, String, _, _>(
                        "fetch-policy",
                        QueryOptions::new(),
                        |_| async {
                            Ok(Fetched::with_policy(9_u32, CachePolicy::Ttl { ttl_ms: 0 }))
                        },
                    )
                    .await;
                timer(ms(5)).await;
                let stale = client
                    .get_query_state("fetch-policy")
                    .map(|state| state.is_stale);
                FETCH_POLICY_SHOWN.fetch_add(1, Ordering::SeqCst);
                shown.set(format!("{} {:?}", out.is_ok(), stale));
            });
        }))
}

#[test]
fn fetch_query_with_policy_applies_the_returned_policy() {
    // TanStack fetchQuery's cache state follows the policy the fetcher returns: the zero TTL
    // makes the entry stale after the load, which the default TTL would not.
    let mut app = start(Root::of::<FetchPolicyApp>());
    app.wait_for("text:run").unwrap();
    app.press("text:run").unwrap();
    app.run_for(ms(20)).unwrap();
    app.wait_for("text:result true Some(true)").unwrap();
}

#[component]
pub fn MutateVariantsApp() -> impl IntoElement {
    let save = use_mutation_with::<String, String, String, _, _>(
        MutationOptions::new(),
        |v, _| async move { Ok(format!("saved {v}")) },
    );
    let by_ref = String::from("by ref");
    let shared = Arc::new(String::from("arc"));
    view()
        .child(text(move || {
            format!("data {}", save.data().unwrap_or_default())
        }))
        .child(text("run by ref").on_press(move || save.mutate_by_ref(&by_ref)))
        .child(text("run arc").on_press(move || save.mutate_arc(shared.clone())))
}

#[test]
fn mutate_by_ref_and_mutate_arc_run_with_their_variables() {
    // mutate_by_ref borrows the variables, and mutate_arc takes them from a shared Arc. Both
    // reach the mutator with their own value.
    let mut app = start(Root::of::<MutateVariantsApp>());
    app.wait_for("text:run by ref").unwrap();
    app.press("text:run by ref").unwrap();
    app.wait_for("text:data saved by ref").unwrap();
    app.press("text:run arc").unwrap();
    app.wait_for("text:data saved arc").unwrap();
}

#[component]
pub fn UnboundedApp() -> impl IntoElement {
    let default_pages = use_infinite_query::<u32, u32, String, _, _>(
        "unbounded-default",
        InfiniteQueryOptions::new(1_u32, |n: &u32| Some(*n + 1)),
        |n, _| async move { Ok(n) },
    );
    let explicit = use_infinite_query::<u32, u32, String, _, _>(
        "unbounded-explicit",
        InfiniteQueryOptions::new(1_u32, |n: &u32| Some(*n + 1))
            .max_pages(3)
            .unbounded_pages(),
        |n, _| async move { Ok(n) },
    );
    spawn(async move {
        while default_pages.page_count() < 55 || explicit.page_count() < 55 {
            default_pages.fetch_next_page();
            explicit.fetch_next_page();
            timer(ms(2)).await;
        }
    });
    view().child(text(move || {
        format!(
            "default {} explicit {}",
            default_pages.page_count(),
            explicit.page_count()
        )
    }))
}

#[test]
fn unbounded_pages_keeps_every_page_past_the_default_limit() {
    // TanStack has no page limit unless maxPages is set. Both queries reach 55 pages, past the
    // core's old default of 50, and max_pages(3) followed by unbounded_pages() keeps all of them.
    let mut app = start(Root::of::<UnboundedApp>());
    app.run_for(seconds(2)).unwrap();
    app.wait_for("text:default 55 explicit 55").unwrap();
}

static MANUAL_LOADS: AtomicUsize = AtomicUsize::new(0);
static MANUAL_OPTS_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ManualApp() -> impl IntoElement {
    let manual = use_query_manual::<u32, String, _, _>("manual", |_| async {
        Ok::<u32, String>((MANUAL_LOADS.fetch_add(1, Ordering::SeqCst) + 1) as u32)
    });
    let manual_opts =
        use_query_manual_opts::<u32, String, _, _>("manual-opts", QueryOptions::new(), |_| async {
            Ok::<u32, String>((MANUAL_OPTS_LOADS.fetch_add(1, Ordering::SeqCst) + 1) as u32)
        });
    view()
        .child(text(move || {
            format!("manual {}", manual.data().unwrap_or(0))
        }))
        .child(text(move || {
            format!("manual opts {}", manual_opts.data().unwrap_or(0))
        }))
        .child(text("load").on_press(move || manual.refetch()))
        .child(text("load opts").on_press(move || manual_opts.refetch()))
}

#[test]
fn use_query_manual_starts_no_fetch_until_asked() {
    // TanStack's manual observer: nothing loads on mount, and refetch is the only load.
    let mut app = start(Root::of::<ManualApp>());
    app.wait_for("text:manual 0").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert_eq!(
        count(&MANUAL_LOADS),
        0,
        "a manual query starts no load on mount"
    );
    assert_eq!(
        count(&MANUAL_OPTS_LOADS),
        0,
        "neither does the options variant"
    );

    app.press("text:load").unwrap();
    app.wait_for("text:manual 1").unwrap();
    assert_eq!(
        count(&MANUAL_LOADS),
        1,
        "refetch is the load that was asked for"
    );
    assert!(
        app.shows("manual opts 0"),
        "the other manual query stays idle"
    );
    assert_eq!(count(&MANUAL_OPTS_LOADS), 0);
}

static SELECT_LOADS: AtomicUsize = AtomicUsize::new(0);
static SELECT_RUNS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn SelectApp() -> impl IntoElement {
    let id = use_query_select::<(u32, u32), String, u32, _, _>(
        "profile",
        QueryOptions::new(),
        |_| async {
            timer(ms(5)).await;
            let n = SELECT_LOADS.fetch_add(1, Ordering::SeqCst) + 1;
            Ok::<(u32, u32), String>(match n {
                1 => (1, 1),
                2 => (1, 2),
                _ => (2, 3),
            })
        },
        |profile: &(u32, u32)| profile.0,
    );
    view()
        .child(text(move || {
            SELECT_RUNS.fetch_add(1, Ordering::SeqCst);
            format!("id {}", id.get().unwrap_or(0))
        }))
        .child(text("refetch").on_press(move || query_client().refetch_queries("profile")))
}

#[test]
fn use_query_select_binding_wakes_only_when_the_selection_changes() {
    // A binding on the selected id re-runs when the id changes, and not when the data changes
    // under the same id.
    let mut app = start(Root::of::<SelectApp>());
    app.wait_for("text:id 0").unwrap();
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:id 1").unwrap();
    let runs = count(&SELECT_RUNS);

    app.press("text:refetch").unwrap();
    app.run_for(ms(10)).unwrap();
    assert_eq!(
        count(&SELECT_LOADS),
        2,
        "the refetch loaded a new name for the same id"
    );
    assert_eq!(
        count(&SELECT_RUNS),
        runs,
        "the selected id did not change, so the binding stays asleep"
    );

    app.press("text:refetch").unwrap();
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:id 2").unwrap();
    assert_eq!(
        count(&SELECT_RUNS),
        runs + 1,
        "the binding wakes once, for the new id"
    );
}

#[component]
pub fn UnsignalledApp() -> impl IntoElement {
    let plain = use_query_unsignalled::<u32, String, _, _>("unsignalled", || async {
        Ok::<u32, String>(4)
    });
    let with_options = use_query_unsignalled_opts::<u32, String, _, _>(
        "unsignalled-opts",
        QueryOptions::new(),
        || async {
            timer(ms(5)).await;
            Ok::<u32, String>(6)
        },
    );
    view().child(text(move || {
        format!(
            "plain {} options {}",
            plain.data().unwrap_or(0),
            with_options.data().unwrap_or(0)
        )
    }))
}

#[test]
fn use_query_unsignalled_loads_with_a_fetcher_that_takes_no_signal() {
    // The signal-free fetcher variant loads just as a context-taking fetcher does. Both the
    // plain and the options form show their value.
    let mut app = start(Root::of::<UnsignalledApp>());
    app.wait_for("text:plain 4 options 0").unwrap();
    app.run_for(ms(10)).unwrap();
    app.wait_for("text:plain 4 options 6").unwrap();
}

/// A value with no `Clone`, to show that `with_query_data` reads it by reference.
pub struct Blob(Vec<u8>);

static BLOB_RUNS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn WithQueryDataApp() -> impl IntoElement {
    query_client().set_data::<Blob, String>("blob", Blob(vec![1, 2, 3]));
    view()
        .child(text(move || {
            BLOB_RUNS.fetch_add(1, Ordering::SeqCst);
            let len = query_client().with_query_data::<Blob, String, _>("blob", |b| b.0.len());
            format!("blob {}", len.unwrap_or(0))
        }))
        .child(text("other").on_press(move || {
            query_client().set_data::<String, String>("other", "x".to_owned());
        }))
        .child(text("replace").on_press(move || {
            query_client().set_data::<Blob, String>("blob", Blob(vec![1, 2, 3, 4]));
        }))
}

#[test]
fn with_query_data_reads_a_value_that_is_not_clone() {
    // gpui-query with_query_data: a closure reads the data by reference, so the value needs no
    // Clone. A binding on it wakes for its own key's writes only.
    let mut app = start(Root::of::<WithQueryDataApp>());
    app.wait_for("text:blob 3").unwrap();
    let runs = count(&BLOB_RUNS);

    app.press("text:other").unwrap();
    app.run_for(ms(10)).unwrap();
    assert_eq!(
        count(&BLOB_RUNS),
        runs,
        "a write to another key does not wake the blob binding"
    );

    app.press("text:replace").unwrap();
    app.wait_for("text:blob 4").unwrap();
    assert_eq!(
        count(&BLOB_RUNS),
        runs + 1,
        "the blob binding wakes once for its own write"
    );
}

#[component]
pub fn UnobservedSeedApp() -> impl IntoElement {
    query_client().set_data::<String, String>(["seeded-gc"], "seed".to_owned());
    let tick = signal(0_u32);
    spawn(async move {
        loop {
            timer(seconds(100)).await;
            tick.update(|n| *n += 1);
        }
    });
    view().child(text(move || {
        tick.get();
        format!(
            "seeded {}",
            query_client().get_query_state(["seeded-gc"]).is_some()
        )
    }))
}

#[test]
fn an_unobserved_seeded_entry_leaves_after_its_gc_time() {
    // TanStack: seeded data with no observer is removed after gcTime, measured from when it was
    // made. The default gcTime is 300 s.
    let mut app = start(Root::of::<UnobservedSeedApp>());
    app.wait_for("text:seeded true").unwrap();
    app.run_for(seconds(200)).unwrap();
    app.wait_for("text:seeded true").unwrap();
    app.run_for(seconds(200)).unwrap();
    app.wait_for("text:seeded false").unwrap();
}

#[component]
pub fn UnobservedInfiniteApp() -> impl IntoElement {
    // A prefetched infinite query has no observer, so it leaves the cache after its fixed 300 s gc.
    query_client().prefetch_infinite_query::<u32, u32, String, _, _>(
        "gc-infinite",
        InfiniteQueryOptions::new(1_u32, |_: &u32| None),
        |n, _| async move { Ok(n) },
    );
    let tick = signal(0_u32);
    spawn(async move {
        loop {
            timer(seconds(100)).await;
            tick.update(|n| *n += 1);
        }
    });
    view().child(text(move || {
        tick.get();
        format!(
            "listed {}",
            query_client()
                .all_infinite_queries::<u32, u32, String>()
                .len()
        )
    }))
}

#[test]
fn an_unobserved_infinite_entry_leaves_after_its_gc_time() {
    // TanStack: a query with no observer is removed after gcTime, infinite queries included.
    let mut app = start(Root::of::<UnobservedInfiniteApp>());
    app.wait_for("text:listed 1").unwrap();
    app.run_for(seconds(200)).unwrap();
    app.wait_for("text:listed 1").unwrap();
    app.run_for(seconds(200)).unwrap();
    app.wait_for("text:listed 0").unwrap();
}

#[component]
pub fn LiveUnsubscribedApp() -> impl IntoElement {
    query_client().set_data::<String, String>(["live-unsub"], "kept".to_owned());
    let shown = use_query_with::<String, String, _, _>(
        ["live-unsub"],
        QueryOptions::new().subscribed(false).gc(seconds(2)),
        |_| async { Ok("fetched".to_owned()) },
    );
    let tick = signal(0_u32);
    spawn(async move {
        loop {
            timer(ms(500)).await;
            tick.update(|n| *n += 1);
        }
    });
    view()
        .child(text(move || {
            tick.get();
            format!(
                "present {}",
                query_client().get_query_state(["live-unsub"]).is_some()
            )
        }))
        .child(text(move || {
            format!("seen {}", shown.data().unwrap_or_default())
        }))
}

#[test]
fn unsubscribed_entry_leaves_the_cache_while_its_component_lives() {
    // TanStack subscribed(false) does not observe, so its query has no observers and leaves the
    // cache after gcTime, even while the component is still on screen. The component keeps the
    // data it had, and never fetches.
    let mut app = start(Root::of::<LiveUnsubscribedApp>());
    app.wait_for("text:seen kept").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert!(
        app.shows("present true"),
        "the entry is cached within its gc time"
    );

    app.run_for(seconds(3)).unwrap();
    app.wait_for("text:present false").unwrap();
    app.wait_for("text:seen kept").unwrap();
}
