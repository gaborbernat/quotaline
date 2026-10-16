//! Tracks refreshed credentials until their backing store confirms the write.

pub const PERSISTENCE_DETAIL: &str =
    "The refreshed session remains active in memory. Quotaline will retry saving it on the next refresh.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending<C, S> {
    pub credential: C,
    pub replacing: C,
    pub source: S,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved<C, S> {
    pub credential: Option<C>,
    pub source: Option<S>,
    pub issue: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadProbe<C, S> {
    ReadError,
    Resolved(Resolved<C, S>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadedProbe<C, S> {
    Resolved(Resolved<C, S>),
    Retry(Pending<C, S>),
}

pub enum SaveOutcome<C, S> {
    Saved,
    Changed(C, S),
    Removed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveCheck {
    Write,
    Changed,
    Removed,
}

pub fn check_save<C: PartialEq>(current: Option<&C>, expected: &C) -> SaveCheck {
    match current {
        Some(credential) if credential == expected => SaveCheck::Write,
        Some(_) => SaveCheck::Changed,
        None => SaveCheck::Removed,
    }
}

pub struct Recovery<C, S> {
    pending: Option<Pending<C, S>>,
}

impl<C, S> Default for Recovery<C, S> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<C: Clone + PartialEq, S: Clone> Recovery<C, S> {
    pub fn pending(&self) -> Option<Pending<C, S>> {
        self.pending.clone()
    }

    pub fn probe_read_error(&self) -> ReadProbe<C, S> {
        self.pending.as_ref().map_or(ReadProbe::ReadError, |pending| {
            ReadProbe::Resolved(Resolved {
                credential: Some(pending.credential.clone()),
                source: Some(pending.source.clone()),
                issue: Some(pending.detail.clone()),
            })
        })
    }

    pub fn probe_loaded(&mut self, loaded: Option<(C, S)>) -> LoadedProbe<C, S> {
        let Some(pending) = self.pending.take() else {
            return LoadedProbe::Resolved(Self::loaded(loaded));
        };
        if loaded
            .as_ref()
            .is_some_and(|(credential, _)| credential == &pending.credential)
        {
            return LoadedProbe::Resolved(Self::loaded(loaded));
        }
        if loaded
            .as_ref()
            .is_some_and(|(credential, _)| credential == &pending.replacing)
        {
            self.pending = Some(pending.clone());
            return LoadedProbe::Retry(pending);
        }
        LoadedProbe::Resolved(Self::loaded(loaded))
    }

    pub fn finish(&mut self, proposed: Pending<C, S>, outcome: SaveOutcome<C, S>) -> Resolved<C, S> {
        self.pending = None;
        match outcome {
            SaveOutcome::Saved => Resolved {
                credential: Some(proposed.credential),
                source: Some(proposed.source),
                issue: None,
            },
            SaveOutcome::Changed(credential, source) => Resolved {
                credential: Some(credential),
                source: Some(source),
                issue: None,
            },
            SaveOutcome::Removed => Resolved {
                credential: None,
                source: None,
                issue: None,
            },
            SaveOutcome::Failed => {
                let pending = Pending {
                    detail: PERSISTENCE_DETAIL.into(),
                    ..proposed
                };
                let resolved = Resolved {
                    credential: Some(pending.credential.clone()),
                    source: Some(pending.source.clone()),
                    issue: Some(pending.detail.clone()),
                };
                self.pending = Some(pending);
                resolved
            }
        }
    }

    fn loaded(loaded: Option<(C, S)>) -> Resolved<C, S> {
        Resolved {
            credential: loaded.as_ref().map(|(credential, _)| credential.clone()),
            source: loaded.map(|(_, source)| source),
            issue: None,
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/save_recovery/tests.rs"]
mod tests;
