//! Formats text, times, deadlines, and the interface copy both desktops share.

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod deadline;
pub mod json;
pub mod text;
pub mod time;
pub mod ui_copy;

#[cfg(any(test, feature = "test-support"))]
pub mod test_clock;
