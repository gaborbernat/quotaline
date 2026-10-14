//! Exposes transcript snapshots and project names to the macOS interface.

use std::path::Path;

use quotaline_claude::claude_transcripts;

mod profile_cache;
mod scanner;
mod state;
pub use profile_cache::{ClaudeProfileCache, ClaudeProfileResult, claude_profile_cache_interval};

#[uniffi::export]
pub fn claude_project_name(transcript: String, root: String) -> String {
    claude_transcripts::project_name(Path::new(&transcript), Path::new(&root))
}

#[uniffi::export]
pub fn claude_encoded_project_name(name: String) -> String {
    claude_transcripts::encoded_project_name(&name)
}

#[uniffi::export]
pub fn claude_local_transcript_warning(source_id: String, configured_service: Option<String>) -> Option<String> {
    quotaline_macos_platform::claude_discovery::local_transcript_warning(&source_id, configured_service.as_deref())
        .map(str::to_owned)
}

#[cfg(test)]
#[path = "../tests/unit/claude/tests.rs"]
mod tests;
