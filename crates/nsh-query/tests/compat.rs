//! Every public name that existed before the TanStack parity work still compiles
//! and behaves as it did. This file is the check for that.

use std::time::{Duration, SystemTime};

use nsh_core::prelude::*;
use nsh_engine::{Clock, EngineConfig, Root};
use nsh_query::{
    CachePolicy, InfiniteQueryOptions, Mutation, MutationOptions, Query, QueryClient, QueryKey,
    QueryOptions, RefetchTrigger, RequestPolicy, RetryPolicy, query_client, use_infinite_query,
    use_mutation, use_mutation_with, use_query, use_query_by, use_query_with,
};
use nsh_testing::Headless;

#[derive(Clone)]
struct Page {
    next: Option<u32>,
    previous: Option<u32>,
}

fn named_query_options() -> QueryOptions {
    QueryOptions::new()
        .cache(CachePolicy::NoCache)
        .ttl(Duration::from_secs(1))
        .stale_while_revalidate(Duration::from_secs(1), Duration::from_secs(1))
        .retry(RetryPolicy::no_retries())
        .request(RequestPolicy::LatestWins)
        .gc(Duration::from_secs(1))
        .refetch_interval(Duration::from_secs(1))
        .refetch_on_foreground(RefetchTrigger::Never)
        .enabled(|| true)
}

fn use_every_public_name(client: QueryClient) -> impl IntoElement {
    let query: Query<String, String> =
        use_query::<String, String, _, _>("compat", |_| async { Ok("ok".to_owned()) });
    let with_options = use_query_with::<String, String, _, _>(
        "compat-options",
        named_query_options(),
        |_| async { Ok("ok".to_owned()) },
    );
    let by_key = use_query_by::<String, String, _, _, _>(
        || QueryKey::from("compat-by"),
        QueryOptions::new(),
        |_| async { Ok("ok".to_owned()) },
    );

    let save = use_mutation::<u32, u32, String, _, _>(|v, _| async move { Ok(v) });
    let saved = use_mutation_with::<u32, u32, String, _, _>(
        MutationOptions::new()
            .retry(RetryPolicy::no_retries())
            .invalidates("compat")
            .on_mutate(|_: &u32| ())
            .on_success(|_: &u32, _: &u32| {})
            .on_error(|_: &String, _: &u32| {})
            .on_settled(|_: &u32| {}),
        |v, _| async move { Ok(v) },
    );

    let infinite = use_infinite_query::<u32, Page, String, _, _>(
        "compat-pages",
        InfiniteQueryOptions::new(1_u32, |p: &Page| p.next)
            .get_previous(|p: &Page| p.previous)
            .max_pages(3)
            .retry(RetryPolicy::no_retries()),
        |n, _| async move {
            Ok(Page {
                next: Some(n + 1),
                previous: n.checked_sub(1),
            })
        },
    );

    let key_match = query.key() == QueryKey::from("compat");
    let _ = (
        query.with(|_| ()),
        query.data(),
        query.error(),
        query.status(),
        query.is_pending(),
        query.is_fetching(),
        query.is_success(),
        query.is_failure(),
        query.retry_count(),
        query.last_updated_at_ms(),
        query.select(|s: &String| s.len()),
        with_options.is_fetching(),
        by_key.is_success(),
        key_match,
    );
    let _ = (
        save.status(),
        save.is_loading(),
        save.is_success(),
        save.is_failure(),
        save.data(),
        save.error(),
        save.variables(),
        saved.status(),
        infinite.pages(),
        infinite.page_count(),
        infinite.has_next_page(),
        infinite.has_previous_page(),
        infinite.is_fetching_next_page(),
        infinite.is_fetching_previous_page(),
        infinite.status(),
        infinite.is_pending(),
        infinite.error(),
    );
    // Compile-only: these calls change the cache, so the branch never runs.
    if std::hint::black_box(false) {
        infinite.fetch_next_page();
        infinite.fetch_previous_page();
        query.refetch();
        query.set_data("manual".to_owned());
        let _ = query.rollback();
        save.mutate(1);
        saved.reset();
        client.invalidate("compat");
        client.cancel("compat");
        client.remove("compat-gone");
        let _: Option<String> = client.data::<String, String>("compat");
        let _: Option<String> = client.data_of::<String>("compat");
        client.set_data::<String, String>("compat", "seeded".to_owned());
        client.set_data_of::<String>("compat-of", "seeded".to_owned());
        client.prefetch("compat-prefetch", QueryOptions::new(), |_| async {
            Ok::<String, String>("pre".to_owned())
        });
    }
    text(move || format!("compat {}", query.data().unwrap_or_default()))
}

#[component]
pub fn CompatApp() -> impl IntoElement {
    let client = query_client();
    let _: Mutation<u32, u32, String> =
        use_mutation::<u32, u32, String, _, _>(|v, _| async move { Ok(v) });
    use_every_public_name(client)
}

#[test]
fn existing_public_names_still_work() {
    let mut config = EngineConfig::new(Root::of::<CompatApp>());
    config.clock = Clock::manual(SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_258_000));
    let mut app = Headless::start(config).unwrap();
    app.wait_for("text:compat ok").unwrap();
}
