use nsh_core::prelude::*;

/// A value with no `Clone` or `PartialEq` that bindings can track: the core's
/// state machines hold user types, and `Signal::update` needs `Clone`.
pub(crate) struct Tracked<R: 'static> {
    value: Stored<R>,
    version: Signal<u64>,
}

impl<R> Clone for Tracked<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for Tracked<R> {}

impl<R: 'static> Tracked<R> {
    pub(crate) fn new(value: R) -> Self {
        Self {
            value: stored(value),
            version: signal(0),
        }
    }

    pub(crate) fn with<T>(&self, f: impl FnOnce(&R) -> T) -> T {
        self.version.get();
        self.value.with(|value| f(value))
    }

    pub(crate) fn with_untracked<T>(&self, f: impl FnOnce(&R) -> T) -> T {
        self.value.with(|value| f(value))
    }

    pub(crate) fn update(&self, f: impl FnOnce(&mut R)) {
        self.value.with(f);
        self.version.update(|v| *v = v.wrapping_add(1));
    }
}
