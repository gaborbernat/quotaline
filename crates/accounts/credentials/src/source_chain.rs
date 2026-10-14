//! Selects a credential source from ordered platform reads.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceState {
    Missing,
    Present,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSelection {
    pub present: Option<usize>,
    pub error: Option<usize>,
}

pub fn first_available(count: usize, mut read: impl FnMut(usize) -> SourceState) -> SourceSelection {
    scan_first(count, &mut read)
}

fn scan_first(count: usize, read: &mut dyn FnMut(usize) -> SourceState) -> SourceSelection {
    let mut error = None;
    for index in 0..count {
        match read(index) {
            SourceState::Present => {
                return SourceSelection {
                    present: Some(index),
                    error,
                };
            }
            SourceState::Failed => {
                error.get_or_insert(index);
            }
            SourceState::Missing => {}
        }
    }
    SourceSelection { present: None, error }
}

pub fn save_target(count: usize, read: impl FnMut(usize) -> SourceState) -> Option<usize> {
    (count != 0).then(|| first_available(count, read).present.unwrap_or(0))
}

pub fn last_error(count: usize, mut read: impl FnMut(usize) -> SourceState) -> SourceSelection {
    scan_last(count, &mut read)
}

fn scan_last(count: usize, read: &mut dyn FnMut(usize) -> SourceState) -> SourceSelection {
    let mut selection = SourceSelection {
        present: None,
        error: None,
    };
    for index in 0..count {
        match read(index) {
            SourceState::Present => {
                selection.present.get_or_insert(index);
            }
            SourceState::Failed => selection.error = Some(index),
            SourceState::Missing => {}
        };
    }
    selection
}

#[cfg(test)]
#[path = "../tests/unit/source_chain.rs"]
mod tests;
