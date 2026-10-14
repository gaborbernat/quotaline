//! Lists the supported providers and their refresh policy.

pub const MAXIMUM_REFRESH_MINUTES: u32 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Definition {
    pub id: &'static str,
    pub name: &'static str,
    pub code: &'static str,
    pub login: &'static str,
    pub minimum_minutes: u32,
    pub default_minutes: u32,
    pub supports_analytics: bool,
}

pub const CLAUDE: Definition = Definition {
    id: "claude",
    name: "Claude",
    code: "CC",
    login: "claude auth login",
    minimum_minutes: 2,
    default_minutes: 5,
    supports_analytics: true,
};

pub const CODEX: Definition = Definition {
    id: "codex",
    name: "Codex",
    code: "CX",
    login: "codex login",
    minimum_minutes: 1,
    default_minutes: 2,
    supports_analytics: true,
};

pub const GEMINI: Definition = Definition {
    id: "gemini",
    name: "Gemini",
    code: "GM",
    login: "gemini",
    minimum_minutes: 1,
    default_minutes: 2,
    supports_analytics: false,
};

pub const ANTIGRAVITY: Definition = Definition {
    id: "antigravity",
    name: "Antigravity",
    code: "AG",
    login: "Sign in through Antigravity",
    minimum_minutes: 1,
    default_minutes: 2,
    supports_analytics: false,
};

pub const CURSOR: Definition = Definition {
    id: "cursor",
    name: "Cursor",
    code: "CU",
    login: "cursor-agent login",
    minimum_minutes: 1,
    default_minutes: 5,
    supports_analytics: false,
};

pub const COPILOT: Definition = Definition {
    id: "copilot",
    name: "GitHub Copilot",
    code: "CP",
    login: "copilot login",
    minimum_minutes: 1,
    default_minutes: 5,
    supports_analytics: false,
};

pub const ALL: [Definition; 6] = [CLAUDE, CODEX, GEMINI, ANTIGRAVITY, CURSOR, COPILOT];

pub const IDS: [&str; 6] = [ALL[0].id, ALL[1].id, ALL[2].id, ALL[3].id, ALL[4].id, ALL[5].id];

pub const CODES: [&str; 6] = [
    ALL[0].code,
    ALL[1].code,
    ALL[2].code,
    ALL[3].code,
    ALL[4].code,
    ALL[5].code,
];

pub const REFRESH_POLICY: [(&str, u32, u32); 6] = [
    (IDS[0], ALL[0].minimum_minutes, ALL[0].default_minutes),
    (IDS[1], ALL[1].minimum_minutes, ALL[1].default_minutes),
    (IDS[2], ALL[2].minimum_minutes, ALL[2].default_minutes),
    (IDS[3], ALL[3].minimum_minutes, ALL[3].default_minutes),
    (IDS[4], ALL[4].minimum_minutes, ALL[4].default_minutes),
    (IDS[5], ALL[5].minimum_minutes, ALL[5].default_minutes),
];

pub fn definition(id: &str) -> Option<&'static Definition> {
    ALL.iter().find(|provider| provider.id == id)
}

pub fn name(id: &str) -> &'static str {
    definition(id).map_or("Provider", |provider| provider.name)
}

pub fn login(id: &str) -> &'static str {
    definition(id).map_or("", |provider| provider.login)
}

#[cfg(test)]
#[path = "../tests/unit/tests.rs"]
mod tests;
