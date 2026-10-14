//! Identifies local Antigravity servers from process paths and arguments.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalRole {
    Cli,
    LanguageServer { csrf_token: String },
}

pub fn candidate(path: &str) -> bool {
    let name = basename(path);
    matches!(name, "agy" | "antigravity-cli")
        || name.starts_with("language_server")
        || name.starts_with("language-server")
}

pub fn role(path: &str, arguments: &[&str]) -> Option<LocalRole> {
    let name = basename(path);
    if matches!(name, "agy" | "antigravity-cli") {
        return Some(LocalRole::Cli);
    }
    if !name.starts_with("language_server") && !name.starts_with("language-server") {
        return None;
    }
    let directory = argument(arguments, "--app_data_dir")?;
    let directory = basename(directory);
    if !directory.eq_ignore_ascii_case("antigravity") && !directory.eq_ignore_ascii_case("antigravity-ide") {
        return None;
    }
    Some(LocalRole::LanguageServer {
        csrf_token: argument(arguments, "--csrf_token")?.into(),
    })
}

fn basename(path: &str) -> &str {
    path.trim_end_matches('/').rsplit('/').next().unwrap_or_default()
}

fn argument<'a>(arguments: &'a [&str], name: &str) -> Option<&'a str> {
    arguments.iter().enumerate().find_map(|(index, argument)| {
        argument
            .strip_prefix(name)
            .and_then(|value| value.strip_prefix('='))
            .or_else(|| (*argument == name).then(|| arguments.get(index + 1).copied()).flatten())
    })
}

#[cfg(test)]
#[path = "../tests/unit/local/tests.rs"]
mod tests;
