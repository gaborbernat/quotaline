//! Exposes parsed status template tokens to the macOS renderer.

use quotaline_status_bar::TemplateToken;
use quotaline_status_bar::parse_template;

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum StatusTemplateTokenKind {
    Text,
    Placeholder,
    Newline,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct StatusTemplateToken {
    pub kind: StatusTemplateTokenKind,
    pub value: String,
}

#[uniffi::export]
pub fn parse_status_template(template: String) -> Vec<StatusTemplateToken> {
    parse_template(&template)
        .into_iter()
        .map(|token| match token {
            TemplateToken::Text(value) => StatusTemplateToken {
                kind: StatusTemplateTokenKind::Text,
                value,
            },
            TemplateToken::Placeholder(value) => StatusTemplateToken {
                kind: StatusTemplateTokenKind::Placeholder,
                value,
            },
            TemplateToken::Newline => StatusTemplateToken {
                kind: StatusTemplateTokenKind::Newline,
                value: String::new(),
            },
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/template/tests.rs"]
mod tests;
