//! Reads Cursor application and agent credentials from their local stores.

use std::{collections::BTreeMap, path::Path};

use quotaline_storage::read_optional_file;
use rusqlite::{Connection, OpenFlags, types::ValueRef};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorAuth {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub email: Option<String>,
    pub membership_type: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorOrigin {
    App,
    Agent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorDiscovery {
    pub auth: CursorAuth,
    pub origin: CursorOrigin,
}

#[derive(Debug, Error)]
pub enum CursorSourceError {
    #[error("cannot open Cursor state: {0}")]
    Open(rusqlite::Error),
    #[error("cannot query Cursor state: {0}")]
    Query(rusqlite::Error),
    #[error("cannot read Cursor state: {0}")]
    Read(rusqlite::Error),
    #[error("cannot read Cursor agent credentials: {0}")]
    File(std::io::Error),
    #[error("Cursor agent credentials are not JSON: {0}")]
    Json(serde_json::Error),
}

fn load_cursor_state(path: &Path) -> Result<Option<CursorAuth>, CursorSourceError> {
    if !path.exists() {
        return Ok(None);
    }
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(CursorSourceError::Open)?;
    let mut query = db
        .prepare("SELECT key,value FROM ItemTable WHERE key LIKE 'cursorAuth/%'")
        .map_err(CursorSourceError::Query)?;
    let values: BTreeMap<String, String> = query
        .query_map([], |row| {
            let raw = row.get_ref(1)?;
            let value = match raw {
                ValueRef::Text(bytes) | ValueRef::Blob(bytes) => std::str::from_utf8(bytes)
                    .map(str::to_owned)
                    .map_err(|error| rusqlite::Error::FromSqlConversionFailure(1, raw.data_type(), Box::new(error)))?,
                _ => row.get(1)?,
            };
            Ok((row.get(0)?, value))
        })
        .map_err(CursorSourceError::Read)?
        .collect::<rusqlite::Result<_>>()
        .map_err(CursorSourceError::Read)?;
    Ok(values
        .get("cursorAuth/accessToken")
        .filter(|token| !token.is_empty())
        .map(|token| CursorAuth {
            access_token: token.clone(),
            refresh_token: values.get("cursorAuth/refreshToken").cloned(),
            email: values.get("cursorAuth/cachedEmail").cloned(),
            membership_type: values.get("cursorAuth/stripeMembershipType").cloned(),
        }))
}

fn load_cursor_agent(path: &Path) -> Result<Option<CursorAuth>, CursorSourceError> {
    let Some(bytes) = read_optional_file(path).map_err(CursorSourceError::File)? else {
        return Ok(None);
    };
    let document: Value = serde_json::from_slice(&bytes).map_err(CursorSourceError::Json)?;
    Ok(document["accessToken"].as_str().map(|access_token| CursorAuth {
        access_token: access_token.into(),
        refresh_token: document["refreshToken"].as_str().map(str::to_owned),
        email: document["email"].as_str().map(str::to_owned),
        membership_type: document["membershipType"].as_str().map(str::to_owned),
    }))
}

pub fn discover_cursor(state_path: &Path, agent_path: &Path) -> Result<Option<CursorDiscovery>, CursorSourceError> {
    let state_error = match load_cursor_state(state_path) {
        Ok(Some(auth)) => {
            return Ok(Some(CursorDiscovery {
                auth,
                origin: CursorOrigin::App,
            }));
        }
        Ok(None) => None,
        Err(error) => Some(error),
    };
    match load_cursor_agent(agent_path) {
        Ok(Some(auth)) if !auth.access_token.is_empty() => Ok(Some(CursorDiscovery {
            auth,
            origin: CursorOrigin::Agent,
        })),
        Ok(_) => state_error.map_or(Ok(None), Err),
        Err(error) => Err(state_error.unwrap_or(error)),
    }
}

#[cfg(test)]
#[path = "../tests/unit/cursor/tests.rs"]
mod tests;
