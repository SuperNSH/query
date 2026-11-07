#![cfg(feature = "persist")]
//! Persistence options: dehydrate and hydrate options, max_age, buster, and use_is_restoring.

use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    DehydrateOptions, HydrateOptions, MutationOptions, PersistOptions, QueryConfig, QueryOptions,
    dehydrate_with, persist_to_storage_with, query_client, restore_from_storage_with,
    use_is_restoring, use_mutation_with, use_query_config,
};
use nsh_testing::Headless;

fn start(root: Root) -> Headless {
    let mut config = EngineConfig::new(root);
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    Headless::start(config).unwrap()
}

#[component]
pub fn OptionsApp() -> impl IntoElement {
    let kept = use_query_config::<String, String, _, _>(
        "opt-kept",
        QueryConfig::<String, String>::from(QueryOptions::new()).persist(),
        |_| async { Ok("kept".to_owned()) },
    );
    let dropped = use_query_config::<String, String, _, _>(
        "opt-dropped",
        QueryConfig::<String, String>::from(QueryOptions::new()).persist(),
        |_| async { Ok("dropped".to_owned()) },
    );
    let _save = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new().mutation_key(nsh_query::QueryKey::from(["opt-mut"])),
        |v, _| async move { Ok(v) },
    );
    let result = signal(String::from("none"));
    view()
        .child(text(move || format!("result {}", result.get())))
        .child(text(move || {
            format!("restoring {}", use_is_restoring().get())
        }))
        .child(text(move || {
            format!(
                "loaded {} {}",
                kept.data().unwrap_or_default(),
                dropped.data().unwrap_or_default()
            )
        }))
        .child(text("options").on_press(move || {
            let client = query_client();
            // Keep the "kept" query only, and list every mutation.
            let state = dehydrate_with(
                &client,
                &DehydrateOptions::new()
                    .should_dehydrate_query(|s| s.key.parts()[0].as_ref() != "opt-dropped")
                    .should_dehydrate_mutation(|_| true),
            );
            result.set(format!(
                "dehydrated {} {}",
                state.queries.len(),
                state.mutations.len()
            ));
        }))
        .child(text("save").on_press(move || {
            let saved = persist_to_storage_with(
                &query_client(),
                "opt-test",
                &PersistOptions::new().buster("v1"),
            )
            .is_ok();
            result.set(format!("saved {saved}"));
        }))
        .child(text("old").on_press(move || {
            // Two seconds later, with a one-second limit: too old.
            let ok = restore_from_storage_with(
                &query_client(),
                "opt-test",
                &PersistOptions::new()
                    .max_age(Duration::from_secs(1))
                    .buster("v1"),
                &HydrateOptions::new(),
            );
            result.set(format!("old {ok}"));
        }))
        .child(text("buster").on_press(move || {
            let ok = restore_from_storage_with(
                &query_client(),
                "opt-test",
                &PersistOptions::new().buster("v2"),
                &HydrateOptions::new(),
            );
            result.set(format!("buster {ok}"));
        }))
        .child(text("fresh").on_press(move || {
            let ok = restore_from_storage_with(
                &query_client(),
                "opt-test",
                &PersistOptions::new().buster("v1"),
                &HydrateOptions::new()
                    .query_defaults(QueryOptions::new().gc(Duration::from_secs(9))),
            );
            let defaults = query_client()
                .get_query_defaults(nsh_query::QueryKey::from(["opt-kept"]))
                .map(|o| o.gc.as_secs());
            result.set(format!("fresh {ok} {defaults:?}"));
        }))
}

#[test]
fn persistence_options_select_restore_and_expire() {
    // TanStack dehydrate/hydrate options, persist maxAge and buster, and useIsRestoring.
    let mut app = start(Root::of::<OptionsApp>());
    app.wait_for("text:loaded kept dropped").unwrap();
    app.wait_for("text:restoring false").unwrap();
    app.press("text:options").unwrap();
    app.wait_for("text:result dehydrated 1 1").unwrap();
    app.press("text:save").unwrap();
    app.wait_for("text:result saved true").unwrap();
    app.run_for(Duration::from_secs(2)).unwrap();
    app.press("text:old").unwrap();
    app.wait_for("text:result old false").unwrap();
    app.press("text:buster").unwrap();
    app.wait_for("text:result buster false").unwrap();
    app.press("text:fresh").unwrap();
    app.wait_for("text:result fresh true Some(9)").unwrap();
}
