use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    MutationOptions, QueryOptions, RefetchTrigger, online_manager, use_mutation_with_context,
    use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

static NET_LOADS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn OfflineApp() -> impl IntoElement {
    let query = use_query_with::<usize, String, _, _>("offline", QueryOptions::new(), |_| async {
        Ok(NET_LOADS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    view()
        .child(text(move || format!("data {}", query.data().unwrap_or(0))))
        .child(text(move || format!("fetch {:?}", query.fetch_status())))
        .child(text(move || format!("paused {}", query.is_paused())))
        .child(text("refetch").on_press(move || query.refetch()))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("online").on_press(|| online_manager().set_online(true)))
}

#[test]
fn offline_pauses_and_online_resumes() {
    let mut app = start(Root::of::<OfflineApp>());
    app.wait_for("text:data 1").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:refetch").unwrap();
    app.wait_for("text:fetch Paused").unwrap();
    assert!(app.shows("paused true"));
    app.run_for(Duration::from_secs(5)).unwrap();
    assert_eq!(NET_LOADS.load(Ordering::SeqCst), 1);
    app.press("text:online").unwrap();
    app.wait_for("text:data 2").unwrap();
    assert!(app.shows("fetch Idle"));
    assert!(app.shows("paused false"));
}

static RECONNECT_ALWAYS: AtomicUsize = AtomicUsize::new(0);
static RECONNECT_LAZY: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn ReconnectApp() -> impl IntoElement {
    let always = use_query_with::<usize, String, _, _>(
        "reconnect-always",
        QueryOptions::new().refetch_on_reconnect(RefetchTrigger::Always),
        |_| async { Ok(RECONNECT_ALWAYS.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    let lazy = use_query_with::<usize, String, _, _>(
        "reconnect-lazy",
        QueryOptions::new().stale_time(Duration::from_secs(60)),
        |_| async { Ok(RECONNECT_LAZY.fetch_add(1, Ordering::SeqCst) + 1) },
    );
    view()
        .child(text(move || {
            format!("always {}", always.data().unwrap_or(0))
        }))
        .child(text(move || format!("lazy {}", lazy.data().unwrap_or(0))))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("online").on_press(|| online_manager().set_online(true)))
}

#[test]
fn reconnect_refetches_when_asked() {
    let mut app = start(Root::of::<ReconnectApp>());
    app.wait_for("text:always 1").unwrap();
    app.wait_for("text:lazy 1").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:online").unwrap();
    app.wait_for("text:always 2").unwrap();
    assert!(app.shows("lazy 1"));
    assert_eq!(RECONNECT_LAZY.load(Ordering::SeqCst), 1);
}

static SAVES: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn PausedSaveApp() -> impl IntoElement {
    let save = use_mutation_with_context::<u32, u32, String, (), _, _>(
        MutationOptions::new(),
        |value, _| async move {
            SAVES.fetch_add(1, Ordering::SeqCst);
            Ok(value)
        },
    );
    view()
        .child(text(move || format!("status {:?}", save.status())))
        .child(text(move || format!("paused {}", save.is_paused())))
        .child(text(move || format!("saved {}", save.data().unwrap_or(0))))
        .child(text("save").on_press(move || save.mutate(5)))
        .child(text("offline").on_press(|| online_manager().set_online(false)))
        .child(text("online").on_press(|| online_manager().set_online(true)))
}

#[test]
fn mutation_pauses_offline_and_resumes() {
    let mut app = start(Root::of::<PausedSaveApp>());
    app.wait_for("text:status Idle").unwrap();
    app.press("text:offline").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:paused true").unwrap();
    assert_eq!(SAVES.load(Ordering::SeqCst), 0);
    app.press("text:online").unwrap();
    app.wait_for("text:saved 5").unwrap();
    assert_eq!(SAVES.load(Ordering::SeqCst), 1);
    assert!(app.shows("paused false"));
}
