use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;

use gpui_query::{QueryError, QueryKey};
use nsh_core::prelude::*;

use crate::options::QueryOptions;
use crate::query::Entry;

pub(crate) trait AnyEntry {
    fn key(&self) -> QueryKey;
    /// Marks the data stale, and reloads it now when something observes it.
    fn invalidate(&self);
    fn cancel(&self);
    fn dispose(&self);
    fn as_any(&self) -> &dyn Any;
}

type EntryId = (TypeId, QueryKey);

#[derive(Default)]
pub(crate) struct Store {
    pub(crate) entries: RefCell<HashMap<EntryId, Rc<dyn AnyEntry>>>,
}

/// The surface's query cache. Entries are keyed by value type and [`QueryKey`],
/// so one key can hold two types without colliding.
#[derive(Clone, Copy)]
pub struct QueryClient(Signal<Store>);

/// The client of the current surface. Every call returns the same cache.
pub fn query_client() -> QueryClient {
    QueryClient(global::<Store>())
}

impl QueryClient {
    pub(crate) fn lookup<T: Clone + 'static>(&self, id: &EntryId) -> Option<T> {
        self.0.with_untracked(|store| {
            store
                .entries
                .borrow()
                .get(id)
                .and_then(|e| e.as_any().downcast_ref::<T>().cloned())
        })
    }

    pub(crate) fn insert(&self, id: EntryId, entry: Rc<dyn AnyEntry>) {
        self.0.with_untracked(|store| {
            store.entries.borrow_mut().insert(id, entry);
        });
    }

    pub(crate) fn evict(&self, id: &EntryId) {
        let removed = self
            .0
            .with_untracked(|store| store.entries.borrow_mut().remove(id));
        if let Some(entry) = removed {
            entry.dispose();
        }
    }

    fn matching(&self, prefix: &QueryKey) -> Vec<(EntryId, Rc<dyn AnyEntry>)> {
        self.0.with_untracked(|store| {
            store
                .entries
                .borrow()
                .iter()
                .filter(|(_, e)| e.key().starts_with(prefix))
                .map(|(id, e)| (id.clone(), e.clone()))
                .collect()
        })
    }

    /// Marks every entry under `prefix` stale and reloads the observed ones.
    /// A load already running is replaced, so its older answer is ignored.
    pub fn invalidate(&self, prefix: impl Into<QueryKey>) {
        for (_, entry) in self.matching(&prefix.into()) {
            entry.invalidate();
        }
    }

    /// Stops the loads running under `prefix`. Data they already loaded is kept.
    pub fn cancel(&self, prefix: impl Into<QueryKey>) {
        for (_, entry) in self.matching(&prefix.into()) {
            entry.cancel();
        }
    }

    /// Drops every entry under `prefix`. Components still observing one keep it
    /// until they go, and a new observer starts from nothing.
    pub fn remove(&self, prefix: impl Into<QueryKey>) {
        for (id, _) in self.matching(&prefix.into()) {
            self.evict(&id);
        }
    }

    /// The cached value of `key`, untracked. `None` when nothing is cached.
    pub fn data<T: Clone + 'static, E: 'static>(&self, key: impl Into<QueryKey>) -> Option<T> {
        Entry::<T, E>::find(self, &key.into()).and_then(|entry| entry.data_untracked())
    }

    /// The cached value of `key` with the default error type.
    pub fn data_of<T: Clone + 'static>(&self, key: impl Into<QueryKey>) -> Option<T> {
        self.data::<T, QueryError>(key)
    }

    /// Writes `data` into the cache as a fresh result, creating the entry if needed.
    pub fn set_data<T: 'static, E: 'static>(&self, key: impl Into<QueryKey>, data: T) {
        let key = key.into();
        Entry::<T, E>::get_or_create(self, &key, &QueryOptions::default()).set_fresh(data);
    }

    /// [`Self::set_data`] with the default error type.
    pub fn set_data_of<T: 'static>(&self, key: impl Into<QueryKey>, data: T) {
        self.set_data::<T, QueryError>(key, data);
    }

    /// Loads `key` into the cache without an observer, unless it is still fresh.
    pub fn prefetch<T, E, F, Fut>(
        &self,
        key: impl Into<QueryKey>,
        options: QueryOptions,
        fetcher: F,
    ) where
        T: 'static,
        E: 'static,
        F: Fn(crate::QueryContext) -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + 'static,
    {
        let entry = Entry::<T, E>::get_or_create(self, &key.into(), &options);
        entry.set_fetcher(Rc::new(move |context| Box::pin(fetcher(context))));
        entry.fetch(gpui_query::QueryFetchMode::Normal);
    }
}
