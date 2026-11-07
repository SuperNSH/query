use std::rc::Rc;
use std::time::Duration;

use gpui_query::{
    CachePolicy, NetworkMode, QueryError, QueryKey, QuerySignal, RefetchTrigger, RequestPolicy,
    RetryPolicy,
};
use nsh_core::JsonValue;

/// What a fetcher gets: the key it loads and a flag set once its result is no longer wanted.
#[derive(Clone, Debug)]
pub struct QueryContext {
    pub key: QueryKey,
    pub signal: QuerySignal,
    /// The `QueryOptions::meta` of the query, if one was set.
    pub meta: Option<JsonValue>,
    /// The client the query runs under.
    pub client: crate::client::QueryClient,
    /// For an infinite query, which end the page is for. `None` for a query.
    /// The page's param is the fetcher's first argument.
    pub direction: Option<PageDirection>,
}

/// Which end of an infinite query a page is loaded for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageDirection {
    /// After the last loaded page.
    Next,
    /// Before the first loaded page.
    Previous,
}

/// How one query caches, retries, refetches and pauses. `Default` is a 60 s
/// TTL, three retries with backoff, in-flight deduplication, a five minute GC
/// time and online-only fetching.
#[derive(Clone)]
pub struct QueryOptions {
    pub cache: CachePolicy,
    pub retry: RetryPolicy,
    pub request: RequestPolicy,
    /// How long an entry with no observers stays cached.
    pub gc: Duration,
    /// Refetch this often while observed and focused.
    pub refetch_interval: Option<Duration>,
    /// What to do when the app returns to the foreground.
    pub refetch_on_foreground: RefetchTrigger,
    pub(crate) enabled: Option<Rc<dyn Fn() -> bool>>,
    pub(crate) refetch_interval_fn: Option<Rc<dyn Fn() -> Option<Duration>>>,
    pub(crate) refetch_interval_in_background: bool,
    pub(crate) refetch_on_mount: RefetchTrigger,
    pub(crate) refetch_on_reconnect: RefetchTrigger,
    pub(crate) retry_on_mount: bool,
    pub(crate) network_mode: NetworkMode,
    pub(crate) initial_data_updated_at: Option<u64>,
    pub(crate) keep_previous_data: bool,
    pub(crate) meta: Option<JsonValue>,
    pub(crate) subscribed: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            cache: CachePolicy::default(),
            retry: RetryPolicy::default(),
            request: RequestPolicy::IgnoreWhileLoading,
            gc: Duration::from_secs(300),
            refetch_interval: None,
            refetch_on_foreground: RefetchTrigger::IfStale,
            enabled: None,
            refetch_interval_fn: None,
            refetch_interval_in_background: false,
            refetch_on_mount: RefetchTrigger::IfStale,
            refetch_on_reconnect: RefetchTrigger::IfStale,
            retry_on_mount: true,
            network_mode: NetworkMode::Online,
            initial_data_updated_at: None,
            keep_previous_data: false,
            meta: None,
            subscribed: true,
        }
    }
}

impl QueryOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache(mut self, policy: CachePolicy) -> Self {
        self.cache = policy;
        self
    }

    pub fn ttl(self, ttl: Duration) -> Self {
        self.cache(CachePolicy::Ttl {
            ttl_ms: ttl.as_millis() as u64,
        })
    }

    /// TanStack's `staleTime`: data is fresh for this long after it loads.
    pub fn stale_time(self, stale: Duration) -> Self {
        self.ttl(stale)
    }

    /// Fresh for `ttl`, then served while revalidating for another `stale`.
    pub fn stale_while_revalidate(self, ttl: Duration, stale: Duration) -> Self {
        self.cache(CachePolicy::StaleWhileRevalidate {
            ttl_ms: ttl.as_millis() as u64,
            stale_ms: stale.as_millis() as u64,
        })
    }

    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    pub fn request(mut self, policy: RequestPolicy) -> Self {
        self.request = policy;
        self
    }

    pub fn gc(mut self, after: Duration) -> Self {
        self.gc = after;
        self
    }

    /// TanStack's `gcTime`, the same as [`Self::gc`].
    pub fn gc_time(self, after: Duration) -> Self {
        self.gc(after)
    }

    pub fn refetch_interval(mut self, every: Duration) -> Self {
        self.refetch_interval = Some(every);
        self
    }

    /// A refetch interval that can change. Returning `None` stops it.
    pub fn refetch_interval_fn(mut self, every: impl Fn() -> Option<Duration> + 'static) -> Self {
        self.refetch_interval_fn = Some(Rc::new(every));
        self
    }

    /// Keeps the interval running while the app is not focused.
    pub fn refetch_interval_in_background(mut self, on: bool) -> Self {
        self.refetch_interval_in_background = on;
        self
    }

    pub fn refetch_on_foreground(mut self, trigger: RefetchTrigger) -> Self {
        self.refetch_on_foreground = trigger;
        self
    }

    /// TanStack's `refetchOnWindowFocus`: on the return to the foreground.
    pub fn refetch_on_window_focus(self, trigger: RefetchTrigger) -> Self {
        self.refetch_on_foreground(trigger)
    }

    /// What a mount does when data is already cached.
    pub fn refetch_on_mount(mut self, trigger: RefetchTrigger) -> Self {
        self.refetch_on_mount = trigger;
        self
    }

    /// When the app comes back online.
    pub fn refetch_on_reconnect(mut self, trigger: RefetchTrigger) -> Self {
        self.refetch_on_reconnect = trigger;
        self
    }

    /// Whether a mount retries a query whose last load failed.
    pub fn retry_on_mount(mut self, on: bool) -> Self {
        self.retry_on_mount = on;
        self
    }

    /// Whether loads wait while the app is offline. `Online` pauses every
    /// attempt, `OfflineFirst` pauses only retries, `Always` never pauses.
    pub fn network_mode(mut self, mode: NetworkMode) -> Self {
        self.network_mode = mode;
        self
    }

    /// When `initial_data` was last fetched, which decides whether it is fresh.
    pub fn initial_data_updated_at(mut self, at_ms: u64) -> Self {
        self.initial_data_updated_at = Some(at_ms);
        self
    }

    /// Keeps showing the previous key's data while a new key loads, and marks
    /// it as placeholder data.
    pub fn keep_previous_data(mut self) -> Self {
        self.keep_previous_data = true;
        self
    }

    /// Whether the observer follows the entry: `false` reads the cache under the
    /// current key, and never fetches, observes or refetches.
    pub fn subscribed(mut self, on: bool) -> Self {
        self.subscribed = on;
        self
    }

    /// Free-form metadata for the app's own use.
    pub fn meta(mut self, meta: JsonValue) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Nothing loads while `when` is false. It is tracked, so reading a signal works.
    pub fn enabled(mut self, when: impl Fn() -> bool + 'static) -> Self {
        self.enabled = Some(Rc::new(when));
        self
    }

    /// Fills each field this value left at its default from `base`. Used to
    /// layer client defaults under a call's options.
    pub(crate) fn over(mut self, base: &QueryOptions) -> Self {
        let d = QueryOptions::default();
        if self.cache == d.cache {
            self.cache = base.cache;
        }
        if self.retry == d.retry {
            self.retry = base.retry.clone();
        }
        if self.request == d.request {
            self.request = base.request;
        }
        if self.gc == d.gc {
            self.gc = base.gc;
        }
        if self.refetch_interval.is_none() {
            self.refetch_interval = base.refetch_interval;
        }
        if self.refetch_on_foreground == d.refetch_on_foreground {
            self.refetch_on_foreground = base.refetch_on_foreground;
        }
        if self.enabled.is_none() {
            self.enabled = base.enabled.clone();
        }
        if self.refetch_interval_fn.is_none() {
            self.refetch_interval_fn = base.refetch_interval_fn.clone();
        }
        if !self.refetch_interval_in_background {
            self.refetch_interval_in_background = base.refetch_interval_in_background;
        }
        if self.refetch_on_mount == d.refetch_on_mount {
            self.refetch_on_mount = base.refetch_on_mount;
        }
        if self.refetch_on_reconnect == d.refetch_on_reconnect {
            self.refetch_on_reconnect = base.refetch_on_reconnect;
        }
        if self.retry_on_mount == d.retry_on_mount {
            self.retry_on_mount = base.retry_on_mount;
        }
        if self.network_mode == d.network_mode {
            self.network_mode = base.network_mode;
        }
        if self.initial_data_updated_at.is_none() {
            self.initial_data_updated_at = base.initial_data_updated_at;
        }
        if !self.keep_previous_data {
            self.keep_previous_data = base.keep_previous_data;
        }
        if self.meta.is_none() {
            self.meta = base.meta.clone();
        }
        if self.subscribed == d.subscribed {
            self.subscribed = base.subscribed;
        }
        self
    }
}

/// A retry predicate over the failure count and the error.
pub(crate) type RetryIf<E> = Rc<dyn Fn(u32, &E) -> bool>;

/// A retry delay over the failure count and the error.
pub(crate) type RetryDelay<E> = Rc<dyn Fn(u32, &E) -> Duration>;

/// Whether two values are the same for structural sharing.
pub(crate) type DataEqual<T> = Rc<dyn Fn(&T, &T) -> bool>;

/// Placeholder data, computed from the previous data if there is any.
pub(crate) type Placeholder<T> = Rc<dyn Fn(Option<&T>) -> Option<T>>;

/// A value's serializer pair, set by `QueryConfig::persist`.
pub(crate) struct Persister<T> {
    #[cfg_attr(not(feature = "persist"), allow(dead_code))]
    pub(crate) ser: fn(&T) -> Option<JsonValue>,
    pub(crate) de: fn(JsonValue) -> Option<T>,
}

impl<T> Clone for Persister<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Persister<T> {}

/// A query's options plus the ones that name its value or error type. Use it
/// with `use_query_config` when a retry predicate, a placeholder or initial
/// data needs `T` or `E`.
pub struct QueryConfig<T: 'static, E: 'static = QueryError> {
    pub options: QueryOptions,
    pub(crate) initial_data: Option<Rc<dyn Fn() -> T>>,
    pub(crate) placeholder_data: Option<Placeholder<T>>,
    pub(crate) retry_if: Option<RetryIf<E>>,
    pub(crate) retry_delay: Option<RetryDelay<E>>,
    pub(crate) persist: Option<Persister<T>>,
    pub(crate) equal: Option<DataEqual<T>>,
    pub(crate) throw: Option<fn(&E) -> String>,
}

impl<T, E> Clone for QueryConfig<T, E> {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
            initial_data: self.initial_data.clone(),
            placeholder_data: self.placeholder_data.clone(),
            retry_if: self.retry_if.clone(),
            retry_delay: self.retry_delay.clone(),
            persist: self.persist,
            equal: self.equal.clone(),
            throw: self.throw,
        }
    }
}

impl<T, E> Default for QueryConfig<T, E> {
    fn default() -> Self {
        Self::from(QueryOptions::default())
    }
}

impl<T, E> From<QueryOptions> for QueryConfig<T, E> {
    fn from(options: QueryOptions) -> Self {
        Self {
            options,
            initial_data: None,
            placeholder_data: None,
            retry_if: None,
            retry_delay: None,
            persist: None,
            equal: None,
            throw: None,
        }
    }
}

impl<T: 'static, E: 'static> QueryConfig<T, E> {
    pub fn new() -> Self {
        Self::default()
    }

    /// TanStack's `retry` as a function of the failure count and the error.
    /// It replaces the retry policy.
    pub fn retry_if(mut self, retry: impl Fn(u32, &E) -> bool + 'static) -> Self {
        self.retry_if = Some(Rc::new(retry));
        self
    }

    /// TanStack's `retryDelay` as a function of the failure count and the error.
    pub fn retry_delay(mut self, delay: impl Fn(u32, &E) -> Duration + 'static) -> Self {
        self.retry_delay = Some(Rc::new(delay));
        self
    }

    /// Data a new entry starts with, fresh as of `initial_data_updated_at`.
    /// It is only used when nothing is cached.
    pub fn initial_data(mut self, initial: impl Fn() -> T + 'static) -> Self {
        self.initial_data = Some(Rc::new(initial));
        self
    }

    /// Data shown while the entry has none. It gets the previous key's data.
    pub fn placeholder_data(
        mut self,
        placeholder: impl Fn(Option<&T>) -> Option<T> + 'static,
    ) -> Self {
        self.placeholder_data = Some(Rc::new(placeholder));
        self
    }

    /// TanStack's `structuralSharing`, as a comparison. When a load's answer equals
    /// the cached data, the data does not change, so a `data()` binding does not run again.
    pub fn structural_sharing(mut self, equal: impl Fn(&T, &T) -> bool + 'static) -> Self {
        self.equal = Some(Rc::new(equal));
        self
    }

    /// TanStack's `throwOnError`: reading the value of a query that failed (and is not
    /// fetching) panics, so the nearest `catch_error` shows the error.
    pub fn throw_on_error(mut self) -> Self
    where
        E: std::fmt::Debug,
    {
        self.throw = Some(|error: &E| format!("{error:?}"));
        self
    }

    /// Registers this query's value for persistence with `dehydrate`.
    #[cfg(feature = "persist")]
    pub fn persist(mut self) -> Self
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        self.persist = Some(Persister {
            ser: |value| serde_json::to_value(value).ok(),
            de: |value| serde_json::from_value(value).ok(),
        });
        self
    }
}

/// Client-wide defaults for mutations under a key prefix.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MutationDefaults {
    pub retry: Option<RetryPolicy>,
    pub network_mode: Option<NetworkMode>,
    pub gc_time: Option<Duration>,
}
