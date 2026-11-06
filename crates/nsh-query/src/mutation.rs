use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

use gpui_query::{MutationResource, MutationStatus, QueryError, QueryKey, RetryPolicy};
use nsh_core::element::create_root;
use nsh_core::prelude::*;

use crate::client::query_client;
use crate::tracked::Tracked;

type Hook<A> = Option<Rc<dyn Fn(&A)>>;
type Hook2<A, B> = Option<Rc<dyn Fn(&A, &B)>>;

type Mutator<V, T, E> =
    Rc<dyn Fn(V, gpui_query::QuerySignal) -> Pin<Box<dyn Future<Output = Result<T, E>>>>>;

/// Callbacks and retry for a mutation. `Default` does not retry, because a
/// write that failed may still have reached the server.
pub struct MutationOptions<V: 'static, T: 'static, E: 'static = QueryError> {
    retry: RetryPolicy,
    invalidates: Vec<QueryKey>,
    on_mutate: Hook<V>,
    on_success: Hook2<T, V>,
    on_error: Hook2<E, V>,
    on_settled: Hook<V>,
}

impl<V, T, E> Default for MutationOptions<V, T, E> {
    fn default() -> Self {
        Self {
            retry: RetryPolicy::no_retries(),
            invalidates: Vec::new(),
            on_mutate: None,
            on_success: None,
            on_error: None,
            on_settled: None,
        }
    }
}

impl<V, T, E> MutationOptions<V, T, E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// Queries under this prefix are invalidated after a success.
    pub fn invalidates(mut self, prefix: impl Into<QueryKey>) -> Self {
        self.invalidates.push(prefix.into());
        self
    }

    /// Runs before the mutator. The place for an optimistic `Query::set_data`.
    pub fn on_mutate(mut self, f: impl Fn(&V) + 'static) -> Self {
        self.on_mutate = Some(Rc::new(f));
        self
    }

    pub fn on_success(mut self, f: impl Fn(&T, &V) + 'static) -> Self {
        self.on_success = Some(Rc::new(f));
        self
    }

    /// Runs after the last failed attempt. The place for `Query::rollback`.
    pub fn on_error(mut self, f: impl Fn(&E, &V) + 'static) -> Self {
        self.on_error = Some(Rc::new(f));
        self
    }

    /// Runs after a success or a failure, last.
    pub fn on_settled(mut self, f: impl Fn(&V) + 'static) -> Self {
        self.on_settled = Some(Rc::new(f));
        self
    }
}

struct Shared<V: 'static, T: 'static, E: 'static> {
    mutator: Mutator<V, T, E>,
    options: MutationOptions<V, T, E>,
    generation: u64,
    in_flight: usize,
    unmounted: bool,
}

/// A write a component can start. `Copy`; reads are tracked. A mutation in
/// flight finishes even if the component goes, so saving and leaving works.
pub struct Mutation<V: 'static, T: 'static, E: 'static = QueryError> {
    resource: Tracked<MutationResource<V, T, E>>,
    shared: Stored<Shared<V, T, E>>,
    owner: Owner,
}

impl<V, T, E> Clone for Mutation<V, T, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V, T, E> Copy for Mutation<V, T, E> {}

impl<V: Clone + 'static, T: 'static, E: 'static> Mutation<V, T, E> {
    /// Starts the mutation. Starting another one supersedes this one: its
    /// result is dropped, and its callbacks do not run.
    pub fn mutate(&self, variables: V) {
        let mutation = *self;
        self.owner.run(|| {
            spawn(async move {
                let _ = mutation.run(variables).await;
            })
        });
    }

    /// [`Self::mutate`] for a task that wants the result. `None` means a newer
    /// call or a `reset` superseded it.
    pub async fn mutate_async(&self, variables: V) -> Option<Result<T, E>>
    where
        T: Clone,
        E: Clone,
    {
        let settled = self.run(variables).await?;
        self.resource.with_untracked(|r| match settled {
            Ok(()) => r.data().cloned().map(Ok),
            Err(()) => r.error().cloned().map(Err),
        })
    }

    async fn run(self, variables: V) -> Option<Result<(), ()>> {
        let generation = self.shared.with(|s| {
            s.generation += 1;
            s.in_flight += 1;
            s.generation
        });
        let (mutator, retry, on_mutate) = self.shared.with(|s| {
            (
                s.mutator.clone(),
                s.options.retry.clone(),
                s.options.on_mutate.clone(),
            )
        });
        if let Some(on_mutate) = on_mutate {
            on_mutate(&variables);
        }
        self.resource.update(|r| r.begin(variables.clone()));

        let mut attempt = 0;
        let outcome = loop {
            let signal = self
                .resource
                .with_untracked(|r| r.signal().cloned().unwrap_or_default());
            let result = mutator(variables.clone(), signal).await;
            if self.shared.with(|s| s.generation) != generation {
                break None;
            }
            match result {
                Err(_) if retry.should_retry(attempt) => {
                    let delay = retry.delay_for_attempt(attempt);
                    attempt += 1;
                    self.resource.update(|r| {
                        r.increment_retry();
                        r.prepare_retry();
                    });
                    timer(Duration::from_millis(delay)).await;
                }
                other => break Some(other),
            }
        };

        let settled = match outcome {
            None => None,
            Some(Ok(data)) => {
                let (on_success, prefixes) = self
                    .shared
                    .with(|s| (s.options.on_success.clone(), s.options.invalidates.clone()));
                if let Some(on_success) = on_success {
                    on_success(&data, &variables);
                }
                self.resource.update(|r| r.complete_success(data));
                let client = query_client();
                for prefix in prefixes {
                    client.invalidate(prefix);
                }
                Some(Ok(()))
            }
            Some(Err(error)) => {
                let on_error = self.shared.with(|s| s.options.on_error.clone());
                if let Some(on_error) = on_error {
                    on_error(&error, &variables);
                }
                self.resource.update(|r| r.complete_failure(error));
                Some(Err(()))
            }
        };
        if settled.is_some() {
            let on_settled = self.shared.with(|s| s.options.on_settled.clone());
            if let Some(on_settled) = on_settled {
                on_settled(&variables);
            }
        }

        let dispose = self.shared.with(|s| {
            s.in_flight -= 1;
            s.unmounted && s.in_flight == 0
        });
        if dispose {
            self.owner.dispose();
        }
        settled
    }

    pub fn status(&self) -> MutationStatus {
        self.resource.with(MutationResource::status)
    }

    pub fn is_loading(&self) -> bool {
        self.resource.with(MutationResource::is_loading)
    }

    pub fn is_success(&self) -> bool {
        self.resource.with(MutationResource::is_success)
    }

    pub fn is_failure(&self) -> bool {
        self.resource.with(MutationResource::is_failure)
    }

    pub fn data(&self) -> Option<T>
    where
        T: Clone,
    {
        self.resource.with(|r| r.data().cloned())
    }

    pub fn error(&self) -> Option<E>
    where
        E: Clone,
    {
        self.resource.with(|r| r.error().cloned())
    }

    /// The variables of the latest call, for showing what is being saved.
    pub fn variables(&self) -> Option<V> {
        self.resource.with(|r| r.variables().cloned())
    }

    /// Back to idle. A call in flight is dropped.
    pub fn reset(&self) {
        self.shared.with(|s| s.generation += 1);
        self.resource.update(MutationResource::reset);
    }
}

/// A mutation with default options.
pub fn use_mutation<V, T, E, F, Fut>(mutator: F) -> Mutation<V, T, E>
where
    V: Clone + 'static,
    T: 'static,
    E: 'static,
    F: Fn(V, gpui_query::QuerySignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_mutation_with(MutationOptions::default(), mutator)
}

/// A mutation with options.
pub fn use_mutation_with<V, T, E, F, Fut>(
    options: MutationOptions<V, T, E>,
    mutator: F,
) -> Mutation<V, T, E>
where
    V: Clone + 'static,
    T: 'static,
    E: 'static,
    F: Fn(V, gpui_query::QuerySignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let mutator: Mutator<V, T, E> =
        Rc::new(move |variables, signal| Box::pin(mutator(variables, signal)));
    let retry = options.retry.clone();
    let (owner, (resource, shared)) = create_root(|| {
        (
            Tracked::new(MutationResource::<V, T, E>::new(retry)),
            stored(Shared {
                mutator,
                options,
                generation: 0,
                in_flight: 0,
                unmounted: false,
            }),
        )
    });
    cleanup(move || {
        let idle = shared.with(|s| {
            s.unmounted = true;
            s.in_flight == 0
        });
        if idle {
            owner.dispose();
        }
    });
    Mutation {
        resource,
        shared,
        owner,
    }
}
