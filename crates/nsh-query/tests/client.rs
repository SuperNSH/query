use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    FetchQueryError, MutationOptions, QueryFilters, QueryKey, QueryOptions, QueryType,
    query_client, use_is_fetching, use_is_mutating, use_mutation_state, use_mutation_with,
    use_queries_combined, use_query, use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

static ITEMS: AtomicUsize = AtomicUsize::new(0);
static ONE: AtomicUsize = AtomicUsize::new(0);
static TWO: AtomicUsize = AtomicUsize::new(0);
static THREE: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FiltersApp() -> impl IntoElement {
    let items = use_query_with::<usize, String, _, _>(["items"], QueryOptions::new(), |_| async {
        Ok(ITEMS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    let one =
        use_query_with::<usize, String, _, _>(["items", "1"], QueryOptions::new(), |_| async {
            Ok(ONE.fetch_add(1, Ordering::SeqCst) + 1)
        });
    let two =
        use_query_with::<usize, String, _, _>(["items", "2"], QueryOptions::new(), |_| async {
            Ok(TWO.fetch_add(1, Ordering::SeqCst) + 1)
        });
    // Loaded with no observer, so it is inactive.
    query_client().prefetch(["items", "3"], QueryOptions::new(), |_| async {
        Ok::<usize, String>(THREE.fetch_add(1, Ordering::SeqCst) + 1)
    });
    let client = query_client();
    view()
        .child(text(move || {
            format!(
                "counts {} {} {}",
                items.data().unwrap_or(0),
                one.data().unwrap_or(0),
                two.data().unwrap_or(0)
            )
        }))
        .child(text("exact").on_press(move || {
            client.refetch_queries(QueryFilters {
                query_key: Some(QueryKey::from(["items"])),
                exact: true,
                ..QueryFilters::default()
            });
        }))
        .child(text("inactive").on_press(move || {
            client.refetch_queries(QueryFilters {
                query_type: QueryType::Inactive,
                ..QueryFilters::default()
            });
        }))
        .child(text("fresh").on_press(move || {
            client.refetch_queries(QueryFilters {
                query_key: Some(QueryKey::from(["items", "2"])),
                stale: Some(false),
                ..QueryFilters::default()
            });
        }))
        .child(text("stale").on_press(move || {
            client.refetch_queries(QueryFilters {
                query_key: Some(QueryKey::from(["items", "2"])),
                stale: Some(true),
                ..QueryFilters::default()
            });
        }))
        .child(text("predicate").on_press(move || {
            client.refetch_queries(QueryFilters {
                predicate: Some(Rc::new(|key: &QueryKey| key.parts().len() == 2)),
                ..QueryFilters::default()
            });
        }))
}

#[test]
fn query_filters_exact_type_stale_predicate() {
    let mut app = start(Root::of::<FiltersApp>());
    app.wait_for("text:counts 1 1 1").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(THREE.load(Ordering::SeqCst), 1);

    app.press("text:exact").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(ITEMS.load(Ordering::SeqCst), 2);
    assert_eq!(
        (ONE.load(Ordering::SeqCst), TWO.load(Ordering::SeqCst)),
        (1, 1)
    );

    app.press("text:inactive").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(THREE.load(Ordering::SeqCst), 2);
    assert_eq!(ITEMS.load(Ordering::SeqCst), 2);

    // "two" is fresh, so only the fresh filter matches it.
    app.press("text:stale").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(TWO.load(Ordering::SeqCst), 1);
    app.press("text:fresh").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(TWO.load(Ordering::SeqCst), 2);

    app.press("text:predicate").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(ONE.load(Ordering::SeqCst), 2);
    assert_eq!(TWO.load(Ordering::SeqCst), 3);
    assert_eq!(THREE.load(Ordering::SeqCst), 3);
    assert_eq!(ITEMS.load(Ordering::SeqCst), 2);
}

static FQ_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FetchQueryApp() -> impl IntoElement {
    let result = signal(String::from("none"));
    let client = query_client();
    view()
        .child(text(move || format!("result {}", result.get())))
        .child(text("fetch").on_press(move || {
            spawn(async move {
                let out = client
                    .fetch_query("fq-direct", QueryOptions::new(), |_| async {
                        timer(Duration::from_millis(50)).await;
                        let n = FQ_LOADS.fetch_add(1, Ordering::SeqCst) + 1;
                        Ok::<String, String>(format!("fetched {n}"))
                    })
                    .await;
                result.set(match out {
                    Ok(value) => value,
                    Err(FetchQueryError::Failed(e)) => format!("failed {e}"),
                    Err(FetchQueryError::Cancelled) => "cancelled".to_owned(),
                });
            });
        }))
        .child(text("ensure").on_press(move || {
            spawn(async move {
                let out = client
                    .ensure_query_data("fq-direct", QueryOptions::new(), |_| async {
                        Ok::<String, String>(String::from("should not load"))
                    })
                    .await;
                result.set(match out {
                    Ok(value) => format!("ensured {value}"),
                    Err(_) => "ensure failed".to_owned(),
                });
            });
        }))
}

#[test]
fn fetch_query_and_ensure_query_data() {
    let mut app = start(Root::of::<FetchQueryApp>());
    app.wait_for("text:fetch").unwrap();
    app.press("text:fetch").unwrap();
    app.run_for(Duration::from_millis(50)).unwrap();
    app.wait_for("text:result fetched 1").unwrap();
    app.press("text:ensure").unwrap();
    app.wait_for("text:result ensured fetched 1").unwrap();
    assert_eq!(FQ_LOADS.load(Ordering::SeqCst), 1);
}

#[component]
pub fn UpdaterApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<u32, String>("counter", 1);
    let counter = use_query::<u32, String, _, _>("counter", |_| async { Ok(0) });
    let changed = signal(false);
    view()
        .child(text(move || {
            format!("count {}", counter.data().unwrap_or(0))
        }))
        .child(text(move || format!("changed {}", changed.get())))
        .child(text("bump").on_press(move || {
            let did =
                client.update_query_data::<u32, String>("counter", |prev| prev.map(|n| n + 1));
            changed.set(did);
        }))
        .child(text("noop").on_press(move || {
            let did = client.update_query_data::<u32, String>("counter", |_| None);
            changed.set(did);
        }))
}

#[test]
fn update_query_data_uses_the_updater() {
    let mut app = start(Root::of::<UpdaterApp>());
    app.wait_for("text:count 1").unwrap();
    app.press("text:bump").unwrap();
    app.wait_for("text:count 2").unwrap();
    assert!(app.shows("changed true"));
    app.press("text:noop").unwrap();
    app.wait_for("text:changed false").unwrap();
    assert!(app.shows("count 2"));
}

#[component]
pub fn MultiApp() -> impl IntoElement {
    let client = query_client();
    client.set_data::<u32, String>(["multi", "a"], 1);
    client.set_data::<u32, String>(["multi", "b"], 2);
    let total = signal(3_u32);
    let shown = computed(move || format!("sum {}", total.get()));
    view()
        .child(text(move || shown.get()))
        .child(text(move || {
            format!(
                "keys {}",
                query_client()
                    .get_queries_data::<u32, String>("multi")
                    .len()
            )
        }))
        .child(text("double").on_press(move || {
            let client = query_client();
            client.set_queries_data::<u32, String>("multi", |prev| prev.map(|n| n * 2));
            let sum: u32 = client
                .get_queries_data::<u32, String>("multi")
                .iter()
                .map(|(_, v)| *v)
                .sum();
            total.set(sum);
        }))
}

#[test]
fn set_queries_data_and_get_queries_data() {
    let mut app = start(Root::of::<MultiApp>());
    app.wait_for("text:keys 2").unwrap();
    app.wait_for("text:sum 3").unwrap();
    app.press("text:double").unwrap();
    app.wait_for("text:sum 6").unwrap();
}

static RESET_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ResetApp() -> impl IntoElement {
    let query =
        use_query_with::<String, String, _, _>("reset-me", QueryOptions::new(), |_| async {
            timer(Duration::from_millis(100)).await;
            Ok(format!(
                "loaded {}",
                RESET_LOADS.fetch_add(1, Ordering::SeqCst) + 1
            ))
        });
    view()
        .child(text(move || {
            format!(
                "value {}",
                query.data().unwrap_or_else(|| "none".to_owned())
            )
        }))
        .child(text("edit").on_press(move || query.set_data("edited".to_owned())))
        .child(text("reset").on_press(|| query_client().reset_queries("reset-me")))
}

#[test]
fn reset_queries_goes_back_to_initial() {
    let mut app = start(Root::of::<ResetApp>());
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:value loaded 1").unwrap();
    app.press("text:edit").unwrap();
    app.wait_for("text:value edited").unwrap();
    app.press("text:reset").unwrap();
    app.wait_for("text:value none").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:value loaded 2").unwrap();
}

static API_LOADS: AtomicUsize = AtomicUsize::new(0);
static OTHER_LOADS: AtomicUsize = AtomicUsize::new(0);

fn defaults_child() -> impl IntoElement {
    let api =
        use_query_with::<usize, String, _, _>(["api", "users"], QueryOptions::new(), |_| async {
            Ok(API_LOADS.fetch_add(1, Ordering::SeqCst) + 1)
        });
    let other = use_query_with::<usize, String, _, _>(["other"], QueryOptions::new(), |_| async {
        Ok(OTHER_LOADS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    view()
        .child(text(move || format!("api {}", api.data().unwrap_or(0))))
        .child(text(move || format!("other {}", other.data().unwrap_or(0))))
}

#[component]
pub fn DefaultsApp() -> impl IntoElement {
    query_client().set_query_defaults("api", QueryOptions::new().gc(Duration::from_secs(10)));
    let shown = signal(true);
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(when(move || shown.get(), defaults_child))
}

#[test]
fn query_defaults_apply_by_key_prefix() {
    let mut app = start(Root::of::<DefaultsApp>());
    app.wait_for("text:api 1").unwrap();
    app.wait_for("text:other 1").unwrap();
    app.press("text:toggle").unwrap();
    app.run_for(Duration::from_secs(11)).unwrap();
    app.press("text:toggle").unwrap();
    app.wait_for("text:api 2").unwrap();
    assert!(app.shows("other 1"));
    assert_eq!(OTHER_LOADS.load(Ordering::SeqCst), 1);
}

#[component]
pub fn CountApp() -> impl IntoElement {
    let fetching = use_is_fetching("slow");
    let mutating = use_is_mutating(QueryKey::from("save"));
    let _slow = use_query_with::<usize, String, _, _>("slow", QueryOptions::new(), |_| async {
        timer(Duration::from_millis(100)).await;
        Ok(1_usize)
    });
    let save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key("save"),
        |value, _| async move {
            timer(Duration::from_millis(100)).await;
            Ok(value)
        },
    );
    view()
        .child(text(move || format!("fetching {}", fetching.get())))
        .child(text(move || format!("mutating {}", mutating.get())))
        .child(text("save").on_press(move || save.mutate(1)))
}

#[test]
fn use_is_fetching_and_is_mutating_count() {
    let mut app = start(Root::of::<CountApp>());
    app.wait_for("text:fetching 1").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:fetching 0").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:mutating 1").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:mutating 0").unwrap();
}

#[component]
pub fn StateApp() -> impl IntoElement {
    let first = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key("log"),
        |value, _| async move {
            timer(Duration::from_millis(100)).await;
            Ok(value)
        },
    );
    let second = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key("log"),
        |value, _| async move {
            timer(Duration::from_millis(100)).await;
            Ok(value)
        },
    );
    let states = use_mutation_state(QueryKey::from("log"), |s| format!("{:?}", s.status));
    view()
        .child(text(move || states.get().join(",")))
        .child(text("a").on_press(move || first.mutate(1)))
        .child(text("b").on_press(move || second.mutate(2)))
}

#[test]
fn use_mutation_state_lists_calls() {
    let mut app = start(Root::of::<StateApp>());
    app.wait_for("text:a").unwrap();
    app.press("text:a").unwrap();
    app.press("text:b").unwrap();
    app.wait_for("text:Loading,Loading").unwrap();
    app.run_for(Duration::from_millis(100)).unwrap();
    app.wait_for("text:Success,Success").unwrap();
}

#[component]
pub fn CombineApp() -> impl IntoElement {
    let options = QueryOptions::new();
    let loaded = use_queries_combined(
        vec![("left", options.clone()), ("right", options)],
        |context| async move {
            timer(Duration::from_millis(50)).await;
            Ok::<String, String>(context.key.parts()[0].to_string())
        },
        |queries| queries.iter().filter(|q| q.data().is_some()).count(),
    );
    text(move || format!("loaded {}", loaded.get()))
}

#[test]
fn use_queries_with_combine() {
    let mut app = start(Root::of::<CombineApp>());
    app.wait_for("text:loaded 0").unwrap();
    app.run_for(Duration::from_millis(50)).unwrap();
    app.wait_for("text:loaded 2").unwrap();
}
