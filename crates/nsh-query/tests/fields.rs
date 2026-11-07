use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    MutationOptions, QueryOptions, RetryPolicy, use_mutation_with_context, use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

static LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn FieldsApp() -> impl IntoElement {
    let query = use_query_with::<String, String, _, _>(
        "fields",
        QueryOptions::new().retry(RetryPolicy::no_retries()),
        |_| async {
            // Attempts: fail, succeed, fail again.
            match LOADS.fetch_add(1, Ordering::SeqCst) + 1 {
                1 => Err("first".to_owned()),
                2 => Ok("second".to_owned()),
                _ => Err("third".to_owned()),
            }
        },
    );
    let disabled = use_query_with::<String, String, _, _>(
        "fields-off",
        QueryOptions::new().enabled(|| false),
        |_| async { Ok("never".to_owned()) },
    );
    view()
        .child(text(move || {
            format!(
                "a {} {} {}",
                query.is_loading(),
                query.is_fetched(),
                query.is_stale()
            )
        }))
        .child(text(move || {
            format!(
                "c {} {} {} {}",
                query.is_loading_error(),
                query.is_refetch_error(),
                query.error_update_count(),
                query.error_updated_at() > 0
            )
        }))
        .child(text(move || {
            format!(
                "d {} {} {}",
                query.data_updated_at() > 0,
                query.is_fetched(),
                query.is_error()
            )
        }))
        .child(text(move || {
            format!(
                "e {} {} {}",
                query.is_refetch_error(),
                query.is_loading_error(),
                query.data().unwrap_or_default()
            )
        }))
        .child(text(move || {
            format!(
                "f {} {} {}",
                disabled.is_enabled(),
                disabled.is_stale(),
                disabled.is_fetching()
            )
        }))
        .child(text("refetch").on_press(move || query.refetch()))
}

#[test]
fn result_fields_report_the_load_state() {
    let mut app = start(Root::of::<FieldsApp>());
    // The first attempt fails at once, so the screen first shows the settled error.
    app.wait_for("text:c true false 1 true").unwrap();
    assert!(app.shows("a false true true"));
    app.wait_for("text:f false false false").unwrap();

    app.press("text:refetch").unwrap();
    app.wait_for("text:d true true false").unwrap();
    assert!(app.shows("e false false second"));

    app.press("text:refetch").unwrap();
    app.wait_for("text:e true false second").unwrap();
    assert_eq!(LOADS.load(Ordering::SeqCst), 3);
}

#[component]
pub fn MutationFieldsApp() -> impl IntoElement {
    let save = use_mutation_with_context::<u32, u32, String, String, _, _>(
        MutationOptions::new().on_mutate(|v: &u32| format!("ctx {v}")),
        |_, _| async { Err("boom".to_owned()) },
    );
    view()
        .child(text(move || {
            format!("idle {} pending {}", save.is_idle(), save.is_pending())
        }))
        .child(text(move || {
            format!(
                "failed {} count {} reason {}",
                save.is_failure(),
                save.failure_count(),
                save.failure_reason().unwrap_or_default()
            )
        }))
        .child(text(move || {
            format!(
                "vars {} context {} at {}",
                save.variables().unwrap_or_default(),
                save.context().unwrap_or_default(),
                save.submitted_at() > 0
            )
        }))
        .child(text("save").on_press(move || save.mutate(7)))
}

#[test]
fn mutation_fields_report_the_call() {
    let mut app = start(Root::of::<MutationFieldsApp>());
    app.wait_for("text:idle true pending false").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:failed true count 1 reason boom")
        .unwrap();
    assert!(app.shows("vars 7 context ctx 7 at true"));
}
