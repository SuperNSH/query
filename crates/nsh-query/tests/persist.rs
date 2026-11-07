#![cfg(feature = "persist")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    QueryConfig, QueryOptions, dehydrate, persist_to_storage, query_client, restore_from_storage,
    use_query_config, use_query_with,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

static SAVED_LOADS: AtomicUsize = AtomicUsize::new(0);

fn saved_child() -> impl IntoElement {
    let config = QueryConfig::<String, String>::from(QueryOptions::new()).persist();
    let saved = use_query_config::<String, String, _, _>("saved", config, |_| async {
        Ok(format!(
            "network {}",
            SAVED_LOADS.fetch_add(1, Ordering::SeqCst) + 1
        ))
    });
    let volatile =
        use_query_with::<String, String, _, _>("volatile", QueryOptions::new(), |_| async {
            Ok("not saved".to_owned())
        });
    view()
        .child(text(move || {
            format!("saved {}", saved.data().unwrap_or_default())
        }))
        .child(text(move || {
            format!("volatile {}", volatile.data().unwrap_or_default())
        }))
}

#[component]
pub fn PersistApp() -> impl IntoElement {
    let shown = signal(true);
    let report = signal(String::from("report none"));
    view()
        .child(text("toggle").on_press(move || shown.update(|s| *s = !*s)))
        .child(text(move || report.get()))
        .child(text("save").on_press(move || {
            let client = query_client();
            let count = dehydrate(&client).queries.len();
            let stored = persist_to_storage(&client, "nsh-query-test").is_ok();
            report.set(format!("report saved {count} {stored}"));
        }))
        .child(text("clear").on_press(|| query_client().clear()))
        .child(text("restore").on_press(move || {
            let restored = restore_from_storage(&query_client(), "nsh-query-test");
            report.set(format!("report restored {restored}"));
        }))
        .child(when(move || shown.get(), saved_child))
}

#[test]
fn dehydrate_and_hydrate_round_trip() {
    let mut app = start(Root::of::<PersistApp>());
    app.wait_for("text:saved network 1").unwrap();
    app.wait_for("text:volatile not saved").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:report saved 1 true").unwrap();

    app.press("text:toggle").unwrap();
    app.press("text:clear").unwrap();
    app.press("text:restore").unwrap();
    app.wait_for("text:report restored true").unwrap();

    app.press("text:toggle").unwrap();
    app.wait_for("text:saved network 1").unwrap();
    app.run_for(Duration::from_millis(10)).unwrap();
    assert_eq!(SAVED_LOADS.load(Ordering::SeqCst), 1);
}
