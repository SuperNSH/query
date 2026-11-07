//! NSH's app focus and connectivity, shaped as TanStack's `focusManager` and
//! `onlineManager`.

use std::future::poll_fn;
use std::task::Poll;

use nsh_core::element::{Lifecycle, lifecycle};
use nsh_core::prelude::*;

use crate::client::query_client;

#[derive(Clone, Copy, Default, PartialEq)]
struct Connectivity {
    offline: bool,
}

/// TanStack's `focusManager`. The app counts as focused while it is visible, as
/// TanStack's visibility check does: `Inactive` counts, `Background` does not.
#[derive(Clone, Copy, Debug, Default)]
pub struct FocusManager;

/// The focus manager of the current surface.
pub fn focus_manager() -> FocusManager {
    FocusManager
}

impl FocusManager {
    /// Whether the app is visible. Reading it subscribes the running computation.
    pub fn is_focused(&self) -> bool {
        lifecycle() != Lifecycle::Background
    }
}

/// TanStack's `onlineManager`. Online by default, since NSH 0.4.0 has no
/// network status API; an app reports offline with `set_online(false)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct OnlineManager;

/// The online manager of the current surface.
pub fn online_manager() -> OnlineManager {
    OnlineManager
}

impl OnlineManager {
    /// Whether fetches and mutations may run now. Reading it subscribes.
    pub fn is_online(&self) -> bool {
        !global::<Connectivity>().get().offline
    }

    /// Reports the connection. Going back online resumes paused fetches and
    /// mutations, and refetches queries whose `refetch_on_reconnect` asks.
    pub fn set_online(&self, online: bool) {
        let state = global::<Connectivity>();
        if state.get_untracked().offline == !online {
            return;
        }
        state.set(Connectivity { offline: !online });
        if online {
            query_client().wake_online_waiters();
        }
    }
}

/// Waits until the app is online. A mutation (`resumable`) also resumes when
/// `resume_paused_mutations` is called, even while offline, as TanStack does.
pub(crate) async fn wait_online(resumable: bool) {
    let resumes = query_client().resume_count();
    poll_fn(|cx| {
        let resumed = resumable && query_client().resume_count() != resumes;
        if online_manager().is_online() || resumed {
            Poll::Ready(())
        } else {
            query_client().add_online_waiter(cx.waker().clone());
            Poll::Pending
        }
    })
    .await
}
