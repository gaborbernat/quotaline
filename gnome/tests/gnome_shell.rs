//! Exercises the GNOME service and Shell in isolated sessions.

use std::{
    fs,
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use futures_util::StreamExt;

#[cfg(feature = "fixture-api")]
use base64::Engine;
#[cfg(feature = "fixture-api")]
use sha2::Digest;
#[cfg(feature = "fixture-api")]
use std::io::Read;

#[cfg(feature = "fixture-api")]
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

#[path = "gnome_shell/harness/mod.rs"]
mod harness;
use harness::*;

#[path = "gnome_shell/tests/mod.rs"]
mod tests;
