/// TanStack's `status`: what the query has to show. `Pending` means no data
/// and no settled error yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Status {
    #[default]
    Pending,
    Error,
    Success,
}

/// TanStack's `fetchStatus`: a load is running, is waiting for the network,
/// or nothing is loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FetchStatus {
    Fetching,
    Paused,
    #[default]
    Idle,
}

/// The last settled outcome. It decides `Status` while a refetch runs, so a
/// refetch after an error still reports `Error`, as TanStack does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Outcome {
    #[default]
    Pending,
    Success,
    Error,
}

impl Outcome {
    pub(crate) fn status(self) -> Status {
        match self {
            Self::Pending => Status::Pending,
            Self::Success => Status::Success,
            Self::Error => Status::Error,
        }
    }
}
