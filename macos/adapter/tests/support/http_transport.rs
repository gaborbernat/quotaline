use crate::http::{NativeHttpCompletion, NativeHttpGet, NativeHttpPostForm, NativeHttpPostJson, NativeHttpTransport};
use std::sync::Arc;

mockall::mock! {
    pub Transport {}
    impl NativeHttpTransport for Transport {
        fn start_get(&self, request: NativeHttpGet, completion: Arc<NativeHttpCompletion>);
        fn start_post_json(&self, request: NativeHttpPostJson, completion: Arc<NativeHttpCompletion>);
        fn start_post_form(&self, request: NativeHttpPostForm, completion: Arc<NativeHttpCompletion>);
    }
}
