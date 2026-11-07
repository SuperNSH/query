//! NSH's rule: only the bindings that read a changed signal run again. Each counter
//! counts the runs of one binding, so a test fails if a binding wakes for a value it never read.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    InfiniteQueryOptions, MutationOptions, QueryOptions, use_infinite_query, use_mutation_with,
    use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

fn runs(counter: &AtomicUsize) -> usize {
    counter.load(Ordering::SeqCst)
}

fn tick(counter: &AtomicUsize) {
    counter.fetch_add(1, Ordering::SeqCst);
}

static LOADS: AtomicUsize = AtomicUsize::new(0);
static DATA: AtomicUsize = AtomicUsize::new(0);
static FETCHING: AtomicUsize = AtomicUsize::new(0);
static FAILURES: AtomicUsize = AtomicUsize::new(0);
static UPDATED_ERROR: AtomicUsize = AtomicUsize::new(0);
static OTHER_DATA: AtomicUsize = AtomicUsize::new(0);
static OTHER_FETCHING: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn QueryFieldsApp() -> impl IntoElement {
    let first = use_query_with::<usize, String, _, _>("first", QueryOptions::new(), |_| async {
        timer(Duration::from_millis(10)).await;
        Ok(LOADS.fetch_add(1, Ordering::SeqCst) + 1)
    });
    let second =
        use_query_with::<usize, String, _, _>("second", QueryOptions::new(), |_| async { Ok(7) });
    view()
        .child(text(move || {
            tick(&DATA);
            format!("data {}", first.data().unwrap_or(0))
        }))
        .child(text(move || {
            tick(&FETCHING);
            format!("fetching {}", first.is_fetching())
        }))
        .child(text(move || {
            tick(&FAILURES);
            format!("failures {}", first.failure_count())
        }))
        .child(text(move || {
            tick(&UPDATED_ERROR);
            format!("errored at {}", first.error_updated_at())
        }))
        .child(text(move || {
            tick(&OTHER_DATA);
            format!("other {}", second.data().unwrap_or(0))
        }))
        .child(text(move || {
            tick(&OTHER_FETCHING);
            format!("other fetching {}", second.is_fetching())
        }))
        .child(text("refetch").on_press(move || first.refetch()))
}

#[test]
fn a_binding_on_one_query_field_wakes_only_for_that_field() {
    let mut app = start(Root::of::<QueryFieldsApp>());
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:data 1").unwrap();
    app.wait_for("text:other 7").unwrap();
    let (data, fetching, failures, error_at, other, other_fetching) = (
        runs(&DATA),
        runs(&FETCHING),
        runs(&FAILURES),
        runs(&UPDATED_ERROR),
        runs(&OTHER_DATA),
        runs(&OTHER_FETCHING),
    );

    app.press("text:refetch").unwrap();
    app.wait_for("text:fetching true").unwrap();
    assert_eq!(runs(&DATA), data, "data must not wake when a fetch starts");
    assert_eq!(runs(&FETCHING), fetching + 1);

    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:data 2").unwrap();
    assert_eq!(runs(&DATA), data + 1, "data wakes once, for the new value");
    assert_eq!(runs(&FETCHING), fetching + 2);
    assert_eq!(runs(&FAILURES), failures, "no failure happened");
    assert_eq!(runs(&UPDATED_ERROR), error_at, "no error happened");
    assert_eq!(runs(&OTHER_DATA), other, "another key's data must not wake");
    assert_eq!(
        runs(&OTHER_FETCHING),
        other_fetching,
        "another key is not fetching"
    );
}

static MUTATION_DATA: AtomicUsize = AtomicUsize::new(0);
static MUTATION_PENDING: AtomicUsize = AtomicUsize::new(0);
static MUTATION_FAILURES: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn MutationFieldsApp() -> impl IntoElement {
    let save =
        use_mutation_with::<u32, u32, String, _, _>(MutationOptions::new(), |n, _| async move {
            timer(Duration::from_millis(10)).await;
            Ok(n + 1)
        });
    view()
        .child(text(move || {
            tick(&MUTATION_DATA);
            format!("saved {}", save.data().unwrap_or(0))
        }))
        .child(text(move || {
            tick(&MUTATION_PENDING);
            format!("pending {}", save.is_pending())
        }))
        .child(text(move || {
            tick(&MUTATION_FAILURES);
            format!("mutation failures {}", save.failure_count())
        }))
        .child(text("save").on_press(move || save.mutate(1)))
}

#[test]
fn a_binding_on_one_mutation_field_wakes_only_for_that_field() {
    let mut app = start(Root::of::<MutationFieldsApp>());
    app.wait_for("text:saved 0").unwrap();
    let (data, pending, failures) = (
        runs(&MUTATION_DATA),
        runs(&MUTATION_PENDING),
        runs(&MUTATION_FAILURES),
    );
    app.press("text:save").unwrap();
    app.wait_for("text:pending true").unwrap();
    assert_eq!(runs(&MUTATION_DATA), data, "no data yet");
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:saved 2").unwrap();
    assert_eq!(runs(&MUTATION_DATA), data + 1);
    assert_eq!(runs(&MUTATION_PENDING), pending + 2);
    assert_eq!(runs(&MUTATION_FAILURES), failures, "no failure happened");
}

static PAGE_COUNT: AtomicUsize = AtomicUsize::new(0);
static NEXT_FLAG: AtomicUsize = AtomicUsize::new(0);
static HAS_PREVIOUS: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn InfiniteFieldsApp() -> impl IntoElement {
    let options = InfiniteQueryOptions::new(1_u32, |n: &u32| (*n < 3).then_some(*n + 1));
    let pages =
        use_infinite_query::<u32, u32, String, _, _>("fine-pages", options, |n, _| async move {
            timer(Duration::from_millis(10)).await;
            Ok(n)
        });
    view()
        .child(text(move || {
            tick(&PAGE_COUNT);
            format!("count {}", pages.page_count())
        }))
        .child(text(move || {
            tick(&NEXT_FLAG);
            format!("loading next {}", pages.is_fetching_next_page())
        }))
        .child(text(move || {
            tick(&HAS_PREVIOUS);
            format!("previous {}", pages.has_previous_page())
        }))
        .child(text("next").on_press(move || pages.fetch_next_page()))
}

#[test]
fn a_binding_on_one_infinite_field_wakes_only_for_that_field() {
    let mut app = start(Root::of::<InfiniteFieldsApp>());
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:count 1").unwrap();
    let (count, next, previous) = (runs(&PAGE_COUNT), runs(&NEXT_FLAG), runs(&HAS_PREVIOUS));
    app.press("text:next").unwrap();
    app.wait_for("text:loading next true").unwrap();
    assert_eq!(runs(&PAGE_COUNT), count, "no new page yet");
    app.run_for(Duration::from_millis(20)).unwrap();
    app.wait_for("text:count 2").unwrap();
    assert_eq!(runs(&PAGE_COUNT), count + 1);
    assert_eq!(runs(&NEXT_FLAG), next + 2);
    assert_eq!(
        runs(&HAS_PREVIOUS),
        previous,
        "has_previous_page did not change"
    );
}
