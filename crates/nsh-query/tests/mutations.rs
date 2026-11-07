use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{MutationOptions, use_mutation_with, use_mutation_with_context};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

#[component]
pub fn ContextApp() -> impl IntoElement {
    let log = signal(String::new());
    let options = MutationOptions::new()
        .on_mutate(|v: &u32| format!("ctx {v}"))
        .on_error_ctx(move |_: &String, _: &u32, c: &String| {
            log.update(|s| s.push_str(&format!("error:{c};")));
        })
        .on_settled_ctx(move |_: &u32, c: &String| {
            log.update(|s| s.push_str(&format!("settled:{c};")));
        });
    let save = use_mutation_with_context::<u32, u32, String, String, _, _>(options, |_, _| async {
        Err("boom".to_owned())
    });
    view()
        .child(text(move || format!("log {}", log.get())))
        .child(text("save").on_press(move || save.mutate(7)))
}

#[test]
fn mutation_context_reaches_error_and_settled() {
    let mut app = start(Root::of::<ContextApp>());
    app.wait_for("text:save").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:log error:ctx 7;settled:ctx 7;").unwrap();
}

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);
static DONE: AtomicUsize = AtomicUsize::new(0);

async fn guarded_write(value: u32) -> Result<u32, String> {
    let now = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
    MAX_ACTIVE.fetch_max(now, Ordering::SeqCst);
    timer(Duration::from_millis(100)).await;
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
    DONE.fetch_add(1, Ordering::SeqCst);
    Ok(value)
}

#[component]
pub fn ScopeApp() -> impl IntoElement {
    let first = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().scope("writes"),
        |value, _| guarded_write(value),
    );
    let second = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().scope("writes"),
        |value, _| guarded_write(value),
    );
    view()
        .child(text("first").on_press(move || first.mutate(1)))
        .child(text("second").on_press(move || second.mutate(2)))
}

#[test]
fn mutation_scope_runs_in_series() {
    let mut app = start(Root::of::<ScopeApp>());
    app.wait_for("text:first").unwrap();
    app.press("text:first").unwrap();
    app.press("text:second").unwrap();
    app.run_for(Duration::from_millis(250)).unwrap();
    assert_eq!(DONE.load(Ordering::SeqCst), 2);
    assert_eq!(MAX_ACTIVE.load(Ordering::SeqCst), 1);
}
