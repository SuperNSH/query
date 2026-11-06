use std::rc::Rc;
use std::time::Duration;

use gpui_query::{CachePolicy, QueryKey, QuerySignal, RefetchTrigger, RequestPolicy, RetryPolicy};

/// What a fetcher gets: the key it loads and a flag set once its result is no longer wanted.
#[derive(Clone, Debug)]
pub struct QueryContext {
    pub key: QueryKey,
    pub signal: QuerySignal,
}

/// How one query caches, retries and refetches. `Default` is a 60 s TTL, three
/// retries with backoff, in-flight deduplication and a five minute GC time.
#[derive(Clone)]
pub struct QueryOptions {
    pub cache: CachePolicy,
    pub retry: RetryPolicy,
    pub request: RequestPolicy,
    /// How long an entry with no observers stays cached.
    pub gc: Duration,
    /// Refetch this often while observed and not in the background.
    pub refetch_interval: Option<Duration>,
    /// What to do when the app returns to the foreground.
    pub refetch_on_foreground: RefetchTrigger,
    pub(crate) enabled: Option<Rc<dyn Fn() -> bool>>,
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

    pub fn refetch_interval(mut self, every: Duration) -> Self {
        self.refetch_interval = Some(every);
        self
    }

    pub fn refetch_on_foreground(mut self, trigger: RefetchTrigger) -> Self {
        self.refetch_on_foreground = trigger;
        self
    }

    /// Nothing loads while `when` is false. It is tracked, so reading a signal works.
    pub fn enabled(mut self, when: impl Fn() -> bool + 'static) -> Self {
        self.enabled = Some(Rc::new(when));
        self
    }
}
