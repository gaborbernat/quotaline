//! Declares the Swift HTTP callback used by Rust provider workflows.

use super::{NativeHttpCompletion, NativeHttpGet, NativeHttpPostForm, NativeHttpPostJson};
use std::sync::Arc;

#[uniffi::export(foreign)]
pub trait NativeHttpTransport: Send + Sync {
    fn start_get(&self, request: NativeHttpGet, completion: Arc<NativeHttpCompletion>);
    fn start_post_json(&self, request: NativeHttpPostJson, completion: Arc<NativeHttpCompletion>);
    fn start_post_form(&self, request: NativeHttpPostForm, completion: Arc<NativeHttpCompletion>);
}
