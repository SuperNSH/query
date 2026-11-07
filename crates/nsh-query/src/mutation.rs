use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui_query::{
    MutationResource, MutationStatus, NetworkMode, QueryError, QueryKey, QuerySignal, RetryPolicy,
};
use nsh_core::element::create_root;
use nsh_core::prelude::*;

use crate::cache::MutationEvent;
use crate::client::{QueryClient, query_client};
use crate::diagnostics::MutationDiagnostic;
use crate::managers::{online_manager, wait_online};
use crate::now_ms;
use crate::tracked::Tracked;

type Hook<A> = Option<Rc<dyn Fn(&A)>>;
type Hook2<A, B> = Option<Rc<dyn Fn(&A, &B)>>;
type HookCtx<A, C> = Rc<dyn Fn(&A, &C)>;
type HookCtx2<A, B, C> = Rc<dyn Fn(&A, &B, &C)>;
type MutateFn<V, C> = Rc<dyn Fn(&V) -> C>;
type RetryDelayFn<E> = Rc<dyn Fn(u32, &E) -> Duration>;

/// Copies a value for a caller that wants its own result of a call.
fn dup_value<X: Clone>(value: &X) -> X {
    value.clone()
}

/// Copy functions for the value and the error of a call's own result.
type Dup<T, E> = (fn(&T) -> T, fn(&E) -> E);

/// Per-call callbacks of [`Mutation::mutate_with`]. They run only for the latest
/// call, and only while the component is alive.
#[non_exhaustive]
pub struct MutateOptions<V, T, E> {
    /// Runs after a success.
    pub on_success: Hook2<T, V>,
    /// Runs after the last failed attempt.
    pub on_error: Hook2<E, V>,
    /// Runs after a success or a failure, last.
    pub on_settled: Hook<V>,
}

impl<V, T, E> Default for MutateOptions<V, T, E> {
    fn default() -> Self {
        Self {
            on_success: None,
            on_error: None,
            on_settled: None,
        }
    }
}

impl<V, T, E> MutateOptions<V, T, E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_success(mut self, f: impl Fn(&T, &V) + 'static) -> Self {
        self.on_success = Some(Rc::new(f));
        self
    }

    pub fn on_error(mut self, f: impl Fn(&E, &V) + 'static) -> Self {
        self.on_error = Some(Rc::new(f));
        self
    }

    pub fn on_settled(mut self, f: impl Fn(&V) + 'static) -> Self {
        self.on_settled = Some(Rc::new(f));
        self
    }
}

type Mutator<V, T, E> = Rc<dyn Fn(V, QuerySignal) -> Pin<Box<dyn Future<Output = Result<T, E>>>>>;

/// Callbacks, retry and scheduling for a mutation. `Default` does not retry,
/// because a write that failed may still have reached the server.
///
/// `C` is the context `on_mutate` returns. Call `on_mutate` before any
/// `*_ctx` hook: it changes `C`, and it drops hooks set earlier.
pub struct MutationOptions<V: 'static, T: 'static, E: 'static = QueryError, C: 'static = ()> {
    retry: RetryPolicy,
    retry_delay: Option<RetryDelayFn<E>>,
    throw: Option<fn(&E) -> String>,
    network: NetworkMode,
    gc: Duration,
    mutation_key: Option<QueryKey>,
    scope: Option<String>,
    invalidates: Vec<QueryKey>,
    on_mutate: Option<MutateFn<V, C>>,
    on_success: Hook2<T, V>,
    on_success_ctx: Option<HookCtx2<T, V, C>>,
    on_error: Hook2<E, V>,
    on_error_ctx: Option<HookCtx2<E, V, C>>,
    on_settled: Hook<V>,
    on_settled_ctx: Option<HookCtx<V, C>>,
}

impl<V, T, E> Default for MutationOptions<V, T, E, ()> {
    fn default() -> Self {
        Self {
            retry: RetryPolicy::no_retries(),
            retry_delay: None,
            throw: None,
            network: NetworkMode::Online,
            gc: Duration::from_secs(300),
            mutation_key: None,
            scope: None,
            invalidates: Vec::new(),
            on_mutate: None,
            on_success: None,
            on_success_ctx: None,
            on_error: None,
            on_error_ctx: None,
            on_settled: None,
            on_settled_ctx: None,
        }
    }
}

impl<V, T, E> MutationOptions<V, T, E, ()> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs before the mutator and returns a context for the callbacks that take one.
    /// The place for an optimistic `Query::set_data`.
    pub fn on_mutate<C: 'static>(
        self,
        f: impl Fn(&V) -> C + 'static,
    ) -> MutationOptions<V, T, E, C> {
        MutationOptions {
            retry: self.retry,
            retry_delay: self.retry_delay,
            throw: self.throw,
            network: self.network,
            gc: self.gc,
            mutation_key: self.mutation_key,
            scope: self.scope,
            invalidates: self.invalidates,
            on_mutate: Some(Rc::new(f)),
            on_success: self.on_success,
            on_success_ctx: None,
            on_error: self.on_error,
            on_error_ctx: None,
            on_settled: self.on_settled,
            on_settled_ctx: None,
        }
    }
}

impl<V, T, E, C> MutationOptions<V, T, E, C> {
    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// TanStack's `throwOnError`: reading the error of a failed call panics, so the nearest
    /// `catch_error` shows it.
    pub fn throw_on_error(mut self) -> Self
    where
        E: std::fmt::Debug,
    {
        self.throw = Some(|error: &E| format!("{error:?}"));
        self
    }

    /// TanStack's `retryDelay`, as a function of the failure count and the error.
    pub fn retry_delay(mut self, delay: impl Fn(u32, &E) -> Duration + 'static) -> Self {
        self.retry_delay = Some(Rc::new(delay));
        self
    }

    pub fn network_mode(mut self, mode: NetworkMode) -> Self {
        self.network = mode;
        self
    }

    pub fn gc_time(mut self, after: Duration) -> Self {
        self.gc = after;
        self
    }

    pub fn mutation_key(mut self, key: impl Into<QueryKey>) -> Self {
        self.mutation_key = Some(key.into());
        self
    }

    /// Calls with the same scope run one after another, in the order they started.
    pub fn scope(mut self, id: impl Into<String>) -> Self {
        self.scope = Some(id.into());
        self
    }

    /// Queries under this prefix are invalidated after a success.
    pub fn invalidates(mut self, prefix: impl Into<QueryKey>) -> Self {
        self.invalidates.push(prefix.into());
        self
    }

    pub fn on_success(mut self, f: impl Fn(&T, &V) + 'static) -> Self {
        self.on_success = Some(Rc::new(f));
        self
    }

    pub fn on_success_ctx(mut self, f: impl Fn(&T, &V, &C) + 'static) -> Self {
        self.on_success_ctx = Some(Rc::new(f));
        self
    }

    /// Runs after the last failed attempt. The place for `Query::rollback`.
    pub fn on_error(mut self, f: impl Fn(&E, &V) + 'static) -> Self {
        self.on_error = Some(Rc::new(f));
        self
    }

    pub fn on_error_ctx(mut self, f: impl Fn(&E, &V, &C) + 'static) -> Self {
        self.on_error_ctx = Some(Rc::new(f));
        self
    }

    /// Runs after a success or a failure, last.
    pub fn on_settled(mut self, f: impl Fn(&V) + 'static) -> Self {
        self.on_settled = Some(Rc::new(f));
        self
    }

    pub fn on_settled_ctx(mut self, f: impl Fn(&V, &C) + 'static) -> Self {
        self.on_settled_ctx = Some(Rc::new(f));
        self
    }
}

/// What the client's mutation cache knows about one mutation.
#[derive(Clone, Debug, PartialEq)]
pub struct MutationState {
    pub status: MutationStatus,
    pub is_paused: bool,
    pub submitted_at: u64,
    pub failure_count: u32,
    pub mutation_key: Option<QueryKey>,
    pub scope: Option<String>,
}

pub(crate) trait AnyMutation {
    fn state(&self) -> MutationState;
    fn diagnostic(&self) -> MutationDiagnostic;
}

struct MutMeta<E> {
    submitted_at: u64,
    paused: bool,
    failure_count: u32,
    failure_reason: Option<E>,
    /// Bumped when the data or error changes, the variables change, or the context
    /// changes. Each is its own counter so a reader of one does not wake for another.
    data_epoch: u64,
    variables_epoch: u64,
    context_epoch: u64,
}

impl<E> Default for MutMeta<E> {
    fn default() -> Self {
        Self {
            submitted_at: 0,
            paused: false,
            failure_count: 0,
            failure_reason: None,
            data_epoch: 0,
            variables_epoch: 0,
            context_epoch: 0,
        }
    }
}

struct Shared<V: 'static, T: 'static, E: 'static, C: 'static> {
    mutator: Mutator<V, T, E>,
    options: MutationOptions<V, T, E, C>,
    generation: u64,
    in_flight: usize,
    unmounted: bool,
}

/// The memos over a mutation's cell. Each reader subscribes to the one it reads.
#[derive(Clone, Copy)]
pub(crate) struct MFields {
    data_epoch: Memo<u64>,
    variables_epoch: Memo<u64>,
    context_epoch: Memo<u64>,
    status: Memo<MutationStatus>,
    paused: Memo<bool>,
    failures: Memo<u32>,
    submitted: Memo<u64>,
}

impl MFields {
    fn new<V: 'static, T: 'static, E: 'static>(
        resource: Tracked<MutationResource<V, T, E>>,
        meta: Tracked<MutMeta<E>>,
    ) -> Self {
        Self {
            data_epoch: computed(move || meta.with(|m| m.data_epoch)),
            variables_epoch: computed(move || meta.with(|m| m.variables_epoch)),
            context_epoch: computed(move || meta.with(|m| m.context_epoch)),
            status: computed(move || resource.with(MutationResource::status)),
            paused: computed(move || meta.with(|m| m.paused)),
            failures: computed(move || meta.with(|m| m.failure_count)),
            submitted: computed(move || meta.with(|m| m.submitted_at)),
        }
    }
}

/// A write a component can start. `Copy`; reads are tracked. A mutation in
/// flight finishes even if the component goes, so saving and leaving works.
pub struct Mutation<V: 'static, T: 'static, E: 'static = QueryError, C: 'static = ()> {
    id: u64,
    resource: Tracked<MutationResource<V, T, E>>,
    meta: Tracked<MutMeta<E>>,
    context: Tracked<Option<C>>,
    fields: MFields,
    shared: Stored<Shared<V, T, E, C>>,
    owner: Owner,
}

impl<V, T, E, C> Clone for Mutation<V, T, E, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<V, T, E, C> Copy for Mutation<V, T, E, C> {}

impl<V: 'static, T: 'static, E: 'static, C: 'static> AnyMutation for Mutation<V, T, E, C> {
    fn state(&self) -> MutationState {
        let (paused, submitted_at, failure_count) = self
            .meta
            .with_untracked(|m| (m.paused, m.submitted_at, m.failure_count));
        let (mutation_key, scope) = self
            .shared
            .with(|s| (s.options.mutation_key.clone(), s.options.scope.clone()));
        MutationState {
            status: self.resource.with_untracked(MutationResource::status),
            is_paused: paused,
            submitted_at,
            failure_count,
            mutation_key,
            scope,
        }
    }

    fn diagnostic(&self) -> MutationDiagnostic {
        MutationDiagnostic {
            key: self
                .shared
                .with(|s| s.options.mutation_key.as_ref().map(QueryKey::to_path)),
            status: self.resource.with_untracked(MutationResource::status),
            retry_count: self.resource.with_untracked(MutationResource::retry_count),
        }
    }
}

impl<V: Clone + 'static, T: 'static, E: 'static, C: 'static> Mutation<V, T, E, C> {
    /// Starts the mutation. A newer call supersedes this one for the state and
    /// the callbacks, but this call still runs to completion with its own result.
    pub fn mutate(&self, variables: V) {
        let mutation = *self;
        self.owner.run(|| {
            spawn(async move {
                let _ = mutation.run(variables, None, None).await;
            })
        });
    }

    /// [`Self::mutate`] with the variables borrowed. The value is cloned once for the call.
    pub fn mutate_by_ref(&self, variables: &V) {
        self.mutate(variables.clone());
    }

    /// [`Self::mutate`] with shared variables. The value is copied out of the `Arc` only when
    /// another holder still has it.
    pub fn mutate_arc(&self, variables: Arc<V>) {
        self.mutate(Arc::unwrap_or_clone(variables));
    }

    /// [`Self::mutate`] with per-call callbacks. They run only for the latest
    /// call, and only while the component is alive.
    pub fn mutate_with(&self, variables: V, options: MutateOptions<V, T, E>) {
        let mutation = *self;
        self.owner.run(|| {
            spawn(async move {
                let _ = mutation.run(variables, Some(options), None).await;
            })
        });
    }

    /// [`Self::mutate`] for a task that wants the result. The call resolves with
    /// its own result even when a newer call supersedes it, as TanStack's
    /// `mutateAsync` does. The `Option` is kept for existing callers and is
    /// always `Some`.
    pub async fn mutate_async(&self, variables: V) -> Option<Result<T, E>>
    where
        T: Clone,
        E: Clone,
    {
        self.run(variables, None, Some((dup_value::<T>, dup_value::<E>)))
            .await
    }

    fn generation_is(&self, generation: u64) -> bool {
        self.shared.with(|s| s.generation) == generation
    }

    /// Runs one call and returns its own outcome when `dup` asks for a copy.
    async fn run(
        self,
        variables: V,
        per_call: Option<MutateOptions<V, T, E>>,
        dup: Option<Dup<T, E>>,
    ) -> Option<Result<T, E>> {
        let (generation, scope) = self.shared.with(|s| {
            s.generation += 1;
            s.in_flight += 1;
            (s.generation, s.options.scope.clone())
        });
        let now = now_ms();
        self.meta.update(|m| {
            m.submitted_at = now;
            m.failure_count = 0;
            m.failure_reason = None;
            m.paused = false;
        });
        query_client().touch();
        if let Some(scope) = scope.as_deref() {
            let scope = scope.to_owned();
            poll_fn(|cx| query_client().poll_scope(&scope, cx.waker())).await;
        }
        let own = self.execute(generation, &variables, per_call, dup).await;
        if let Some(scope) = scope.as_deref() {
            query_client().release_scope(scope);
        }
        let retire = self.shared.with(|s| {
            s.in_flight -= 1;
            s.unmounted && s.in_flight == 0
        });
        query_client().touch();
        if retire {
            self.retire();
        }
        own
    }

    /// Whether this call is still the latest one. Only the latest call writes the
    /// state and runs the global callbacks.
    fn is_latest(&self, generation: u64) -> bool {
        self.generation_is(generation)
    }

    fn set_paused(&self, generation: u64, on: bool) {
        if self.is_latest(generation) {
            self.meta.update(|m| m.paused = on);
        }
    }

    async fn execute(
        &self,
        generation: u64,
        variables: &V,
        per_call: Option<MutateOptions<V, T, E>>,
        dup: Option<Dup<T, E>>,
    ) -> Option<Result<T, E>> {
        let (mutator, retry, retry_delay, network, on_mutate) = self.shared.with(|s| {
            (
                s.mutator.clone(),
                s.options.retry.clone(),
                s.options.retry_delay.clone(),
                s.options.network,
                s.options.on_mutate.clone(),
            )
        });
        if self.is_latest(generation) {
            let context = on_mutate.as_ref().map(|f| f(variables));
            // One change, one notification per binding.
            batch(|| {
                self.context.update(|c| *c = context);
                self.meta.update(|m| {
                    m.context_epoch += 1;
                    m.variables_epoch += 1;
                });
                self.resource.update(|r| r.begin(variables.clone()));
            });
            query_client().touch();
        }

        if self.is_latest(generation) {
            query_client().notify_mutation(&self.state(), MutationEvent::Mutate);
        }
        let mut failures = 0_u32;
        let outcome: Result<T, E> = loop {
            let pause = !online_manager().is_online()
                && match network {
                    NetworkMode::Online => true,
                    NetworkMode::OfflineFirst => failures > 0,
                    NetworkMode::Always => false,
                };
            if pause {
                self.set_paused(generation, true);
                query_client().touch();
                wait_online(true).await;
                self.set_paused(generation, false);
            }
            let signal = self
                .resource
                .with_untracked(|r| r.signal().cloned().unwrap_or_default());
            match mutator(variables.clone(), signal).await {
                Ok(data) => break Ok(data),
                Err(error) => {
                    failures += 1;
                    if retry.should_retry(failures - 1) {
                        let delay = match &retry_delay {
                            Some(f) => f(failures - 1, &error),
                            None => Duration::from_millis(retry.delay_for_attempt(failures - 1)),
                        };
                        if self.is_latest(generation) {
                            let mut slot = Some(error);
                            batch(|| {
                                self.resource.update(|r| {
                                    r.increment_retry();
                                    r.prepare_retry();
                                });
                                self.meta.update(|m| {
                                    m.failure_count = failures;
                                    m.failure_reason = slot.take();
                                });
                            });
                        }
                        timer(delay).await;
                        continue;
                    }
                    break Err(error);
                }
            }
        };

        // The copy for the caller is taken before the state takes the value.
        let own = match (&outcome, dup) {
            (Ok(data), Some((copy, _))) => Some(Ok(copy(data))),
            (Err(error), Some((_, copy))) => Some(Err(copy(error))),
            _ => None,
        };
        if !self.is_latest(generation) {
            return own;
        }
        let alive = self.shared.with(|s| !s.unmounted);
        let (on_success, on_success_ctx, on_error, on_error_ctx, prefixes) =
            self.shared.with(|s| {
                (
                    s.options.on_success.clone(),
                    s.options.on_success_ctx.clone(),
                    s.options.on_error.clone(),
                    s.options.on_error_ctx.clone(),
                    s.options.invalidates.clone(),
                )
            });
        let succeeded = outcome.is_ok();
        match outcome {
            Ok(data) => {
                if let Some(on_success) = on_success {
                    on_success(&data, variables);
                }
                if let Some(hook) = on_success_ctx {
                    self.context.with_untracked(|c| {
                        if let Some(c) = c {
                            hook(&data, variables, c);
                        }
                    });
                }
                if let (Some(call), true) =
                    (per_call.as_ref().and_then(|p| p.on_success.clone()), alive)
                {
                    call(&data, variables);
                }
                batch(|| {
                    self.resource.update(|r| r.complete_success(data));
                    self.meta.update(|m| {
                        m.data_epoch += 1;
                        m.failure_count = failures;
                        m.failure_reason = None;
                    });
                });
                let client = query_client();
                for prefix in prefixes {
                    client.invalidate(prefix);
                }
            }
            Err(error) => {
                if let Some(on_error) = on_error {
                    on_error(&error, variables);
                }
                if let Some(hook) = on_error_ctx {
                    self.context.with_untracked(|c| {
                        if let Some(c) = c {
                            hook(&error, variables, c);
                        }
                    });
                }
                if let (Some(call), true) =
                    (per_call.as_ref().and_then(|p| p.on_error.clone()), alive)
                {
                    call(&error, variables);
                }
                batch(|| {
                    self.resource.update(|r| r.complete_failure(error));
                    self.meta.update(|m| {
                        m.data_epoch += 1;
                        m.failure_count = failures;
                        m.failure_reason = None;
                    });
                });
            }
        }
        let (on_settled, on_settled_ctx) = self.shared.with(|s| {
            (
                s.options.on_settled.clone(),
                s.options.on_settled_ctx.clone(),
            )
        });
        if let Some(on_settled) = on_settled {
            on_settled(variables);
        }
        if let Some(hook) = on_settled_ctx {
            self.context.with_untracked(|c| {
                if let Some(c) = c {
                    hook(variables, c);
                }
            });
        }
        if let (Some(call), true) = (per_call.as_ref().and_then(|p| p.on_settled.clone()), alive) {
            call(variables);
        }
        let state = self.state();
        let outcome_event = if succeeded {
            MutationEvent::Success
        } else {
            MutationEvent::Error
        };
        query_client().notify_mutation(&state, outcome_event);
        query_client().notify_mutation(&state, MutationEvent::Settled);
        query_client().touch();
        own
    }

    /// Takes the mutation out of the client's cache and frees its owner.
    fn retire(&self) {
        let gc = self.shared.with(|s| s.options.gc);
        schedule_retire(self.id, self.owner, gc);
    }

    pub fn status(&self) -> MutationStatus {
        self.fields.status.get()
    }

    pub fn is_loading(&self) -> bool {
        self.fields.status.get() == MutationStatus::Loading
    }

    /// TanStack's `isPending`, the same as [`Self::is_loading`].
    pub fn is_pending(&self) -> bool {
        self.is_loading()
    }

    pub fn is_idle(&self) -> bool {
        self.fields.status.get() == MutationStatus::Idle
    }

    pub fn is_success(&self) -> bool {
        self.fields.status.get() == MutationStatus::Success
    }

    pub fn is_failure(&self) -> bool {
        self.fields.status.get() == MutationStatus::Failure
    }

    /// Waiting for the network before its next attempt.
    pub fn is_paused(&self) -> bool {
        self.fields.paused.get()
    }

    pub fn data(&self) -> Option<T>
    where
        T: Clone,
    {
        self.fields.data_epoch.get();
        self.resource.with_untracked(|r| r.data().cloned())
    }

    pub fn error(&self) -> Option<E>
    where
        E: Clone,
    {
        self.fields.data_epoch.get();
        if let Some(message) = self.shared.with(|s| s.options.throw)
            && let Some(text) = self.resource.with_untracked(|r| r.error().map(message))
        {
            panic!("{text}");
        }
        self.resource.with_untracked(|r| r.error().cloned())
    }

    /// Failed attempts in the current call.
    pub fn failure_count(&self) -> u32 {
        self.fields.failures.get()
    }

    /// The error of the latest failed attempt.
    pub fn failure_reason(&self) -> Option<E>
    where
        E: Clone,
    {
        self.fields.failures.get();
        self.fields.data_epoch.get();
        if let Some(reason) = self.meta.with_untracked(|m| m.failure_reason.clone()) {
            return Some(reason);
        }
        if self.is_failure() {
            return self.resource.with_untracked(|r| r.error().cloned());
        }
        None
    }

    /// When the latest call started, in ms since the Unix epoch.
    pub fn submitted_at(&self) -> u64 {
        self.fields.submitted.get()
    }

    /// The context `on_mutate` returned for the latest call.
    pub fn context(&self) -> Option<C>
    where
        C: Clone,
    {
        self.fields.context_epoch.get();
        self.context.with_untracked(|c| c.clone())
    }

    /// The variables of the latest call, for showing what is being saved.
    pub fn variables(&self) -> Option<V>
    where
        V: Clone,
    {
        self.fields.variables_epoch.get();
        self.resource.with_untracked(|r| r.variables().cloned())
    }

    /// Back to idle. A call in flight is dropped.
    pub fn reset(&self) {
        self.shared.with(|s| s.generation += 1);
        batch(|| self.reset_cells());
        query_client().touch();
    }

    fn reset_cells(&self) {
        self.resource.update(MutationResource::reset);
        self.meta.update(|m| {
            // Epochs only grow, so a reader sees the reset as a change.
            let (data, variables, context) = (m.data_epoch, m.variables_epoch, m.context_epoch);
            *m = MutMeta {
                data_epoch: data + 1,
                variables_epoch: variables + 1,
                context_epoch: context + 1,
                ..MutMeta::default()
            };
        });
        self.context.update(|c| *c = None);
        query_client().touch();
    }
}

/// A mutation with default options.
pub fn use_mutation<V, T, E, F, Fut>(mutator: F) -> Mutation<V, T, E>
where
    V: Clone + 'static,
    T: 'static,
    E: 'static,
    F: Fn(V, QuerySignal) -> Fut + 'static,
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
    F: Fn(V, QuerySignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    use_mutation_with_context(options, mutator)
}

/// A mutation whose `on_mutate` returns a context for its callbacks.
pub fn use_mutation_with_context<V, T, E, C, F, Fut>(
    options: MutationOptions<V, T, E, C>,
    mutator: F,
) -> Mutation<V, T, E, C>
where
    V: Clone + 'static,
    T: 'static,
    E: 'static,
    C: 'static,
    F: Fn(V, QuerySignal) -> Fut + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
{
    let client: QueryClient = query_client();
    let mut options = options;
    if let Some(key) = options.mutation_key.clone()
        && let Some(defaults) = client.mutation_defaults_for(&key)
    {
        if options.retry == RetryPolicy::no_retries()
            && let Some(retry) = defaults.retry
        {
            options.retry = retry;
        }
        if options.network == NetworkMode::Online
            && let Some(network) = defaults.network_mode
        {
            options.network = network;
        }
        if let Some(gc) = defaults.gc_time {
            options.gc = gc;
        }
    }
    let mutator: Mutator<V, T, E> =
        Rc::new(move |variables, signal| Box::pin(mutator(variables, signal)));
    let retry = options.retry.clone();
    let id = client.next_mutation_id();
    let (owner, (resource, meta, context, fields, shared)) = create_root(|| {
        let resource = Tracked::new(MutationResource::<V, T, E>::new(retry));
        let meta = Tracked::new(MutMeta::default());
        (
            resource,
            meta,
            Tracked::new(None::<C>),
            MFields::new(resource, meta),
            stored(Shared {
                mutator,
                options,
                generation: 0,
                in_flight: 0,
                unmounted: false,
            }),
        )
    });
    let mutation = Mutation {
        id,
        resource,
        meta,
        context,
        fields,
        shared,
        owner,
    };
    client.register_mutation(id, Rc::new(mutation));
    cleanup(move || {
        let idle = shared.with(|s| {
            s.unmounted = true;
            s.in_flight == 0
        });
        if idle {
            let gc = shared.with(|s| s.options.gc);
            schedule_retire(id, owner, gc);
        }
    });
    mutation
}

/// Removes a settled mutation from the client's cache once its `gc_time` has
/// passed, and frees its owner then. The timer runs under a detached owner, so it
/// outlives the component that made the mutation.
fn schedule_retire(id: u64, owner: Owner, gc: Duration) {
    Owner::detached().run(|| {
        nsh_core::element::set_timeout(gc, move || {
            query_client().unregister_mutation(id);
            owner.dispose();
        })
    });
}
