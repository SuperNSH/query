//! Tests for the third round: TanStack's `client.query`, the `subscribed(false)` gc, and the
//! cache, error, persistence and gpui-query surfaces. Each goes through a real UI on the
//! manual clock and checks the TanStack behavior named in its comment.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    CacheEvent, InfiniteQueryOptions, MutationCacheCallbacks, MutationOptions, QueryCacheCallbacks,
    QueryClientConfig, QueryConfig, QueryKey, QueryOptions, query_client, use_infinite_query,
    use_mutation_with, use_prefetch_infinite_query, use_query_by, use_query_config, use_query_with,
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

static QUERY_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ClientQueryApp() -> impl IntoElement {
    let shown = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", shown.get())))
        .child(text("run").on_press(move || {
            spawn(async move {
                let config =
                    QueryConfig::<String, String>::from(QueryOptions::new().ttl(seconds(60)));
                let first = client
                    .query("client-query", config.clone(), |_| async {
                        Ok::<String, String>(format!(
                            "v{}",
                            QUERY_LOADS.fetch_add(1, Ordering::SeqCst) + 1
                        ))
                    })
                    .await;
                let second = client
                    .query("client-query", config, |_| async {
                        Ok::<String, String>(format!(
                            "v{}",
                            QUERY_LOADS.fetch_add(1, Ordering::SeqCst) + 1
                        ))
                    })
                    .await;
                shown.set(format!("{first:?} {second:?}"));
            });
        }))
}

#[test]
fn client_query_returns_fresh_cached_data_without_fetching() {
    // TanStack client.query: fresh by staleTime returns the cache; the second call fetches nothing.
    let mut app = start(Root::of::<ClientQueryApp>());
    app.wait_for("text:run").unwrap();
    app.press("text:run").unwrap();
    app.wait_for("text:result Ok(\"v1\") Ok(\"v1\")").unwrap();
    assert_eq!(QUERY_LOADS.load(Ordering::SeqCst), 1);
}

static RETRY_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ClientQueryRetryApp() -> impl IntoElement {
    let shown = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", shown.get())))
        .child(text("run").on_press(move || {
            spawn(async move {
                let out = client
                    .query("client-query-retry", QueryOptions::new(), |_| async {
                        RETRY_LOADS.fetch_add(1, Ordering::SeqCst);
                        Err::<String, String>("down".to_owned())
                    })
                    .await;
                shown.set(format!("{out:?}"));
            });
        }))
}

#[test]
fn client_query_is_not_retried_unless_retry_is_set() {
    // TanStack client.query: "the fetch isn't retried unless retry is set". One attempt only.
    let mut app = start(Root::of::<ClientQueryRetryApp>());
    app.wait_for("text:run").unwrap();
    app.press("text:run").unwrap();
    app.wait_for("text:result Err(Failed(\"down\"))").unwrap();
    app.run_for(seconds(10)).unwrap();
    assert_eq!(RETRY_LOADS.load(Ordering::SeqCst), 1);
}

static ENABLED_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ClientQueryEnabledApp() -> impl IntoElement {
    let shown = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", shown.get())))
        .child(text("run").on_press(move || {
            spawn(async move {
                let out = client
                    .query(
                        "client-query-enabled",
                        QueryOptions::new().enabled(|| false),
                        |_| async {
                            ENABLED_LOADS.fetch_add(1, Ordering::SeqCst);
                            Ok::<String, String>("loaded".to_owned())
                        },
                    )
                    .await;
                shown.set(format!("{out:?}"));
            });
        }))
}

#[test]
fn client_query_ignores_enabled_as_tanstack_does() {
    // TanStack client.query does not read `enabled`; only observers do.
    let mut app = start(Root::of::<ClientQueryEnabledApp>());
    app.wait_for("text:run").unwrap();
    app.press("text:run").unwrap();
    app.wait_for("text:result Ok(\"loaded\")").unwrap();
    assert_eq!(ENABLED_LOADS.load(Ordering::SeqCst), 1);
}

#[component]
pub fn ClientQuerySelectApp() -> impl IntoElement {
    let shown = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", shown.get())))
        .child(text("run").on_press(move || {
            spawn(async move {
                let out = client
                    .query_selected(
                        "client-query-select",
                        QueryOptions::new(),
                        |_| async { Ok::<String, String>("hello".to_owned()) },
                        |s: &String| s.len(),
                    )
                    .await;
                shown.set(format!("{out:?}"));
            });
        }))
}

#[test]
fn query_selected_applies_select_before_returning() {
    // TanStack client.query returns the select result.
    let mut app = start(Root::of::<ClientQuerySelectApp>());
    app.wait_for("text:run").unwrap();
    app.press("text:run").unwrap();
    app.wait_for("text:result Ok(5)").unwrap();
}

#[component]
pub fn UnsubscribedGcApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<String, String>(["gc-unsub"], "kept".to_owned());
    let shown = signal(true);
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
                query_client().get_query_state(["gc-unsub"]).is_some()
            )
        }))
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(
            move || shown.get(),
            || {
                let q = use_query_with::<String, String, _, _>(
                    ["gc-unsub"],
                    QueryOptions::new().subscribed(false).gc(seconds(2)),
                    |_| async { Ok("fetched".to_owned()) },
                );
                text(move || format!("seen {}", q.data().unwrap_or_default()))
            },
        ))
}

#[test]
fn unsubscribed_entries_leave_the_cache_after_gc_time() {
    // TanStack: a query with no observer is removed after gcTime. subscribed(false) is not an
    // observer, so its entry must leave after gc_time, not live until the app ends.
    let mut app = start(Root::of::<UnsubscribedGcApp>());
    app.wait_for("text:seen kept").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert!(app.shows("present true"));
    app.run_for(seconds(3)).unwrap();
    app.wait_for("text:present false").unwrap();
}

#[component]
pub fn CacheApiApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<String, String>(["cache-api", "a"], "one".to_owned());
    client.set_data::<String, String>(["cache-api", "b"], "two".to_owned());
    let cache = client.get_query_cache();
    let log = signal(String::new());
    let _sub = cache.subscribe(move |event| {
        let line = match event {
            CacheEvent::Added(key) => format!("+{};", key.parts()[1]),
            CacheEvent::Removed(key) => format!("-{};", key.parts()[1]),
            _ => String::new(),
        };
        log.update(|s| s.push_str(&line));
    });
    view()
        .child(text(move || {
            // Cache reads are plain calls, as in TanStack. Reading the log re-runs this text on events.
            log.get();
            let cache = query_client().get_query_cache();
            format!(
                "find {} all {} get {}",
                cache.find(["cache-api", "a"]).is_some(),
                cache.find_all("cache-api").len(),
                cache.get_all().len()
            )
        }))
        .child(text(move || format!("log {}", log.get())))
        .child(text("remove").on_press(move || {
            query_client().get_query_cache().remove(["cache-api", "a"]);
        }))
        .child(text("add").on_press(move || {
            query_client().set_data::<String, String>(["cache-api", "c"], "three".to_owned());
        }))
        .child(text("clear").on_press(move || {
            query_client().get_query_cache().clear();
        }))
}

#[test]
fn query_cache_finds_lists_subscribes_and_removes() {
    // TanStack QueryCache: find, findAll, getAll, subscribe (add and remove events), remove, clear.
    let mut app = start(Root::of::<CacheApiApp>());
    app.wait_for("text:find true all 2 get 2").unwrap();
    app.press("text:remove").unwrap();
    app.wait_for("text:log -a;").unwrap();
    app.press("text:add").unwrap();
    app.wait_for("text:log -a;+c;").unwrap();
    app.press("text:clear").unwrap();
    app.wait_for("text:find false all 0 get 0").unwrap();
}

static CB_LOG_SET: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn CacheCallbackApp() -> impl IntoElement {
    let log = signal(String::new());
    let client = query_client();
    client.set_config(
        QueryClientConfig::new().query_cache(
            QueryCacheCallbacks::new()
                .on_success(move |s| {
                    log.update(|l| l.push_str(&format!("ok:{};", s.key.parts()[0])))
                })
                .on_error(move |s| {
                    log.update(|l| l.push_str(&format!("err:{};", s.key.parts()[0])))
                })
                .on_settled(move |_| log.update(|l| l.push_str("settled;"))),
        ),
    );
    let _ = CB_LOG_SET.fetch_add(0, Ordering::SeqCst);
    view()
        .child(text(move || format!("log {}", log.get())))
        .child(text("load").on_press(move || {
            spawn(async move {
                let _ = client
                    .fetch_query("cb-ok", QueryOptions::new(), |_| async {
                        Ok::<String, String>("v".to_owned())
                    })
                    .await;
                let _ = client
                    .fetch_query(
                        "cb-bad",
                        QueryOptions::new().retry(nsh_query::RetryPolicy::no_retries()),
                        |_| async { Err::<String, String>("boom".to_owned()) },
                    )
                    .await;
            });
        }))
}

#[test]
fn query_cache_callbacks_fire_for_success_error_and_settled() {
    // TanStack QueryCache config: onSuccess, onError and onSettled for every query.
    let mut app = start(Root::of::<CacheCallbackApp>());
    app.wait_for("text:log ").unwrap();
    app.press("text:load").unwrap();
    app.wait_for("text:log ok:cb-ok;settled;err:cb-bad;settled;")
        .unwrap();
}

#[component]
pub fn MutationCacheApp() -> impl IntoElement {
    let log = signal(String::new());
    let client = query_client();
    client.set_config(
        QueryClientConfig::new().mutation_cache(
            MutationCacheCallbacks::new()
                .on_mutate(move |_| log.update(|l| l.push_str("mutate;")))
                .on_success(move |_| log.update(|l| l.push_str("success;")))
                .on_error(move |_| log.update(|l| l.push_str("error;")))
                .on_settled(move |_| log.update(|l| l.push_str("settled;"))),
        ),
    );
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key(QueryKey::from(["cache-mut"])),
        |v, _| async move {
            if v == 0 {
                Err("zero".to_owned())
            } else {
                Ok(v)
            }
        },
    );
    view()
        .child(text(move || format!("log {}", log.get())))
        .child(text(move || {
            format!(
                "found {}",
                query_client()
                    .get_mutation_cache()
                    .find_all(QueryKey::from(["cache-mut"]))
                    .len()
            )
        }))
        .child(text("ok").on_press(move || save.mutate(1)))
        .child(text("bad").on_press(move || save.mutate(0)))
}

#[test]
fn mutation_cache_callbacks_fire_for_each_point_of_a_call() {
    // TanStack MutationCache config: onMutate, onSuccess, onError and onSettled.
    let mut app = start(Root::of::<MutationCacheApp>());
    app.wait_for("text:log ").unwrap();
    app.press("text:ok").unwrap();
    app.wait_for("text:log mutate;success;settled;").unwrap();
    app.press("text:bad").unwrap();
    app.wait_for("text:log mutate;success;settled;mutate;error;settled;")
        .unwrap();
    app.wait_for("text:found 1").unwrap();
}

#[component]
pub fn FetcherContextApp() -> impl IntoElement {
    query_client().set_default_options(QueryOptions::new().gc(seconds(77)));
    let query = use_query_with::<String, String, _, _>(
        "ctx-query",
        QueryOptions::new().meta(nsh_core::JsonValue::String("q-tag".to_owned())),
        |context| async move {
            let meta = context
                .meta
                .as_ref()
                .and_then(|m| m.as_str())
                .unwrap_or("none")
                .to_owned();
            let gc = context.client.get_default_options().gc.as_secs();
            Ok(format!("{meta}|{gc}"))
        },
    );
    let pages = use_infinite_query::<u32, String, String, _, _>(
        "ctx-pages",
        InfiniteQueryOptions::new(1_u32, |_: &String| Some(2))
            .meta(nsh_core::JsonValue::String("inf-tag".to_owned()))
            .retry(nsh_query::RetryPolicy::no_retries()),
        |n, context| async move {
            let meta = context
                .meta
                .as_ref()
                .and_then(|m| m.as_str())
                .unwrap_or("none")
                .to_owned();
            let gc = context.client.get_default_options().gc.as_secs();
            Ok(format!("{n}:{meta}|{:?}|{gc}", context.direction))
        },
    );
    view()
        .child(text(move || {
            format!("query {}", query.data().unwrap_or_default())
        }))
        .child(text(move || {
            format!(
                "pages {}",
                pages
                    .pages()
                    .iter()
                    .map(|p| p.as_str().to_owned())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }))
}

#[test]
fn fetcher_context_carries_meta_client_and_direction() {
    // TanStack queryFn context: meta and the client. Infinite page fetches also carry the direction.
    let mut app = start(Root::of::<FetcherContextApp>());
    app.wait_for("text:query q-tag|77").unwrap();
    app.wait_for("text:pages 1:inf-tag|Some(Next)|77").unwrap();
}

#[component]
pub fn PrefetchInfiniteApp() -> impl IntoElement {
    use_prefetch_infinite_query(
        "prefetch-inf",
        InfiniteQueryOptions::new(1_u32, |_: &String| None),
        |n, _| async move { Ok::<String, String>(format!("p{n}")) },
    );
    let pages = use_infinite_query::<u32, String, String, _, _>(
        "prefetch-inf",
        InfiniteQueryOptions::new(1_u32, |_: &String| None),
        |n, _| async move { Ok::<String, String>(format!("p{n}")) },
    );
    text(move || {
        format!(
            "prefetched {}",
            pages
                .pages()
                .iter()
                .map(|p| p.as_str().to_owned())
                .collect::<Vec<_>>()
                .join(",")
        )
    })
}

#[test]
fn use_prefetch_infinite_query_serves_a_later_observer() {
    // TanStack usePrefetchInfiniteQuery: the cached first page serves the observer that follows.
    let mut app = start(Root::of::<PrefetchInfiniteApp>());
    app.wait_for("text:prefetched p1").unwrap();
}

#[component]
pub fn MatchMutationApp() -> impl IntoElement {
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key(QueryKey::from(["mm"])),
        |v, _| async move { Ok(v) },
    );
    save.mutate(1);
    text(move || {
        let state = query_client()
            .get_mutation_cache()
            .find(QueryKey::from(["mm"]));
        let matched = state.as_ref().map(|s| {
            nsh_query::match_mutation(&nsh_query::MutationFilters::from(QueryKey::from(["mm"])), s)
        });
        format!("matched {matched:?}")
    })
}

#[test]
fn match_mutation_matches_the_cached_state() {
    // TanStack matchMutation: a mutation's state matches a filter on its key.
    let mut app = start(Root::of::<MatchMutationApp>());
    app.wait_for("text:matched Some(true)").unwrap();
}

#[component]
pub fn ThrowApp() -> impl IntoElement {
    let calls = signal(0_u32);
    let q = use_query_config::<String, String, _, _>(
        "throwing",
        QueryConfig::<String, String>::from(
            QueryOptions::new().retry(nsh_query::RetryPolicy::no_retries()),
        )
        .throw_on_error(),
        move |_| async move {
            let n = calls.get_untracked() + 1;
            calls.set(n);
            if n == 1 {
                Err("boom".to_owned())
            } else {
                Ok("ok".to_owned())
            }
        },
    );
    let boundary = nsh_query::use_query_error_reset_boundary();
    catch_error(
        move || text(move || format!("q {}", q.data().unwrap_or_default())),
        move |error, reset| {
            view()
                .child(text(move || format!("caught {}", error.message)))
                .child(text("retry").on_press(move || {
                    boundary.reset();
                    reset.reset();
                }))
        },
    )
}

#[test]
fn throw_on_error_reaches_catch_error_and_the_reset_boundary_recovers() {
    // TanStack throwOnError: the failed query throws into the nearest boundary. The reset boundary
    // reloads the failed query, and the boundary's view tries again.
    let mut app = start(Root::of::<ThrowApp>());
    app.wait_for("text:caught \"boom\"").unwrap();
    app.press("text:retry").unwrap();
    app.wait_for("text:q ok").unwrap();
}

#[component]
pub fn ThrowMutationApp() -> impl IntoElement {
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().throw_on_error(),
        |_, _| async { Err("nope".to_owned()) },
    );
    save.mutate(1);
    catch_error(
        move || text(move || format!("error {:?}", save.error())),
        |error, _reset| text(move || format!("caught {}", error.message)),
    )
}

#[test]
fn mutation_throw_on_error_reaches_catch_error() {
    // TanStack useMutation throwOnError: reading the failed call's error throws into a boundary.
    let mut app = start(Root::of::<ThrowMutationApp>());
    app.wait_for("text:caught \"nope\"").unwrap();
}

#[component]
pub fn UnsubscribedKeyApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<String, String>(["moved-old"], "old".to_owned());
    client.set_data::<String, String>(["moved-new"], "new".to_owned());
    let key = signal(String::from("moved-old"));
    let tick = signal(0_u32);
    spawn(async move {
        loop {
            timer(ms(500)).await;
            tick.update(|n| *n += 1);
        }
    });
    let shown = use_query_by::<String, String, _, _, _>(
        move || key.get(),
        QueryOptions::new().subscribed(false).gc(seconds(2)),
        |_| async { Ok("fetched".to_owned()) },
    );
    view()
        .child(text(move || {
            tick.get();
            format!(
                "old {}",
                query_client().get_query_state(["moved-old"]).is_some()
            )
        }))
        .child(text(move || {
            format!("seen {}", shown.data().unwrap_or_default())
        }))
        .child(text("move").on_press(move || key.set(String::from("moved-new"))))
}

#[test]
fn unsubscribed_key_change_releases_the_entry_it_left() {
    // An unsubscribed observer that moves to another key stops holding the old entry, which must
    // leave after gc_time.
    let mut app = start(Root::of::<UnsubscribedKeyApp>());
    app.wait_for("text:seen old").unwrap();
    app.press("text:move").unwrap();
    app.wait_for("text:seen new").unwrap();
    app.run_for(seconds(1)).unwrap();
    assert!(app.shows("old true"));
    app.run_for(seconds(3)).unwrap();
    app.wait_for("text:old false").unwrap();
}
