//! Defines a provider's quota reading and the projections every view derives from it.

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub mod notices;
pub mod pace;
pub mod reset_groups;
pub mod restriction;
pub mod snapshot;
mod types;
pub mod usage;

pub use types::*;
