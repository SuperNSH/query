use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{MutationOptions, RetryPolicy, use_mutation_with, use_query};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

static SERVER: AtomicUsize = AtomicUsize::new(5);

#[component]
pub fn SaveApp() -> impl IntoElement {
    let count =
        use_query::<usize, String, _, _>("count", |_| async { Ok(SERVER.load(Ordering::SeqCst)) });
    let options = MutationOptions::new()
        .invalidates("count")
        .on_success(|saved: &usize, _: &usize| assert_eq!(*saved, 9));
    let save = use_mutation_with::<usize, usize, String, _, _>(options, |value, _| async move {
        timer(Duration::from_millis(100)).await;
        SERVER.store(value, Ordering::SeqCst);
        Ok(value)
    });
    view()
        .child(text(move || format!("count {}", count.data().unwrap_or(0))))
        .child(text(move || format!("saving {}", save.is_loading())))
        .child(text("save").on_press(move || save.mutate(9)))
}

#[test]
fn a_success_invalidates_and_the_query_loads_the_new_value() {
    let mut app = start(Root::of::<SaveApp>());
    app.wait_for("text:count 5").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:saving true").unwrap();
    app.run_for(Duration::from_millis(150)).unwrap();
    app.wait_for("text:count 9").unwrap();
    assert!(app.shows("saving false"));
}

static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn OptimisticApp() -> impl IntoElement {
    let liked = use_query::<bool, String, _, _>("liked", |_| async { Ok(false) });
    let options = MutationOptions::new()
        .retry(RetryPolicy::new(1).with_delay(100))
        .on_mutate(move |next: &bool| liked.set_data(*next))
        .on_error(move |_: &String, _: &bool| {
            liked.rollback();
        });
    let like = use_mutation_with::<bool, bool, String, _, _>(options, |_, _| async {
        ATTEMPTS.fetch_add(1, Ordering::SeqCst);
        Err("refused".to_owned())
    });
    view()
        .child(text(move || {
            format!("liked {}", liked.data().unwrap_or(false))
        }))
        .child(text(move || {
            format!("error {}", like.error().unwrap_or_default())
        }))
        .child(text("like").on_press(move || like.mutate(true)))
}

#[test]
fn an_optimistic_write_rolls_back_after_the_last_attempt_fails() {
    let mut app = start(Root::of::<OptimisticApp>());
    app.wait_for("text:liked false").unwrap();
    app.press("text:like").unwrap();
    app.wait_for("text:liked true").unwrap();
    app.run_for(Duration::from_millis(200)).unwrap();
    app.wait_for("text:liked false").unwrap();
    app.wait_for("text:error refused").unwrap();
    assert_eq!(ATTEMPTS.load(Ordering::SeqCst), 2);
}

static SURVIVED: AtomicUsize = AtomicUsize::new(0);

fn leaving_form() -> impl IntoElement {
    let options = MutationOptions::new().on_settled(|_: &u32| {
        SURVIVED.fetch_add(1, Ordering::SeqCst);
    });
    let save = use_mutation_with::<u32, u32, String, _, _>(options, |value, _| async move {
        timer(Duration::from_millis(500)).await;
        Ok(value)
    });
    text("save and leave").on_press(move || save.mutate(1))
}

#[component]
pub fn LeaveApp() -> impl IntoElement {
    let shown = signal(true);
    view()
        .child(text("close").on_press(move || shown.set(false)))
        .child(when(move || shown.get(), leaving_form))
}

#[test]
fn a_mutation_in_flight_finishes_after_its_component_goes() {
    let mut app = start(Root::of::<LeaveApp>());
    app.wait_for("text:save and leave").unwrap();
    app.press("text:save and leave").unwrap();
    app.press("text:close").unwrap();
    app.wait_for_gone("text:save and leave", Duration::from_secs(1))
        .unwrap();
    app.run_for(Duration::from_secs(1)).unwrap();
    assert_eq!(SURVIVED.load(Ordering::SeqCst), 1);
}
