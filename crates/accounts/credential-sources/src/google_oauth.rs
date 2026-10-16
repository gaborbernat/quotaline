//! Extracts installed Google OAuth clients from Antigravity and Gemini CLI sources.

use std::io::Read;

use memchr::memmem;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthClient {
    pub id: String,
    pub secret: String,
}

pub fn antigravity_binary(path: &std::path::Path) -> Option<OAuthClient> {
    antigravity_reader(std::io::BufReader::new(std::fs::File::open(path).ok()?))
}

pub fn antigravity_bytes(bytes: &[u8]) -> Option<OAuthClient> {
    antigravity_reader(bytes)
}

fn antigravity_reader(mut reader: impl Read) -> Option<OAuthClient> {
    const SUFFIX: &[u8] = b".apps.googleusercontent.com";
    const PREFIX: &[u8] = b"GOCSPX-";
    let mut buffer = [0_u8; 65_792];
    let mut tail = 0;
    let mut id = None;
    let mut secret = None;
    loop {
        let read = reader.read(&mut buffer[tail..]).ok()?;
        let len = tail + read;
        for offset in memmem::find_iter(&buffer[..len], SUFFIX) {
            let start = buffer[..offset]
                .iter()
                .rposition(|byte| !byte.is_ascii_alphanumeric() && *byte != b'-' && *byte != b'_')
                .map_or(0, |index| index + 1);
            let candidate = &buffer[start..offset];
            if let Some(separator) = candidate.iter().position(|byte| *byte == b'-')
                && separator > 0
                && separator + 1 < candidate.len()
                && candidate[..separator].iter().all(u8::is_ascii_digit)
            {
                let end = offset + SUFFIX.len();
                id = std::str::from_utf8(&buffer[start..end]).ok().map(str::to_owned);
                break;
            }
        }
        for offset in memmem::find_iter(&buffer[..len], PREFIX) {
            let end = offset + PREFIX.len() + 28;
            if end <= len
                && buffer[offset + PREFIX.len()..end]
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_')
            {
                secret = std::str::from_utf8(&buffer[offset..end]).ok().map(str::to_owned);
                break;
            }
        }
        if let (Some(id), Some(secret)) = (&id, &secret) {
            return Some(OAuthClient {
                id: id.clone(),
                secret: secret.clone(),
            });
        }
        if read == 0 {
            return None;
        }
        tail = len.min(256);
        buffer.copy_within(len - tail..len, 0);
    }
}

pub fn gemini_source(source: &str) -> Option<OAuthClient> {
    Some(OAuthClient {
        id: javascript_assignment(source, "OAUTH_CLIENT_ID")?,
        secret: javascript_assignment(source, "OAUTH_CLIENT_SECRET")?,
    })
}

fn javascript_assignment(source: &str, name: &str) -> Option<String> {
    for (offset, _) in source.match_indices(name) {
        let rest = source[offset + name.len()..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else { continue };
        let rest = rest.trim_start();
        let Some(quote) = rest.chars().next().filter(|quote| *quote == '\'' || *quote == '"') else {
            continue;
        };
        if let Some(value) = rest[1..]
            .split_once(quote)
            .map(|(value, _)| value)
            .filter(|value| !value.is_empty())
        {
            return Some(value.into());
        }
    }
    None
}

#[cfg(test)]
#[path = "../tests/unit/google_oauth/tests.rs"]
mod tests;
