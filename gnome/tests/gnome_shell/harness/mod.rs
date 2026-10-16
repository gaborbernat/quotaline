//! Shares isolated GNOME Shell fixtures across integration scenarios.

use std::{
    fs,
    io::{BufRead, BufReader},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::PermissionsExt,
    },
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};

use futures_util::StreamExt;

#[cfg(feature = "fixture-api")]
use sha2::Digest;
#[cfg(feature = "fixture-api")]
use std::io::{Read, Write};

#[cfg(feature = "fixture-api")]
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

mod services;
pub(crate) use services::*;
mod process;
pub(crate) use process::*;
mod fault_cases;
pub(crate) use fault_cases::*;
mod portal;
pub(crate) use portal::*;
mod history;
pub(crate) use history::*;
mod claude;
pub(crate) use claude::*;
mod fixture;
pub(crate) use fixture::*;
mod cursor_refresh;
pub(crate) use cursor_refresh::*;
mod shell;
pub(crate) use shell::*;
mod notifications;
pub(crate) use notifications::*;
mod credits;
pub(crate) use credits::*;
mod discovery;
pub(crate) use discovery::*;
