//! Answers shared pipeline requests from a script and records what they sent.

use std::future::{Future, ready};
use std::sync::Mutex;

use crate::HttpError;
use crate::transport::{Request, Transport};

pub struct Scripted {
    replies: Mutex<Vec<Result<Vec<u8>, HttpError>>>,
    pub sent: Mutex<Vec<Request>>,
}

impl Scripted {
    pub fn new(replies: Vec<Result<Vec<u8>, HttpError>>) -> Self {
        Self {
            replies: Mutex::new(replies),
            sent: Mutex::new(Vec::new()),
        }
    }
}

impl Transport for Scripted {
    fn send(&self, request: Request) -> impl Future<Output = Result<Vec<u8>, HttpError>> + Send {
        self.sent.lock().unwrap().push(request);
        ready(self.replies.lock().unwrap().remove(0))
    }
}

pub fn unavailable() -> HttpError {
    HttpError::Response {
        status: 503,
        body: String::new(),
        retry_after: None,
    }
}
