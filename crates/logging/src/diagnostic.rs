//! Formats structured diagnostics with secret redaction.

use super::redact;

pub fn message(name: &str, fields: &[(&str, Option<&str>)], home: Option<&str>) -> String {
    let mut message = String::from(name);
    for (key, value) in fields {
        if let Some(value) = value {
            let safe = redact(value, home);
            message.push(' ');
            message.push_str(key);
            message.push('=');
            if safe
                .chars()
                .any(|character| character.is_whitespace() || character == '"' || character == '=')
            {
                message.push('"');
                for character in safe.chars() {
                    if character == '\\' || character == '"' {
                        message.push('\\');
                    }
                    message.push(character);
                }
                message.push('"');
            } else {
                message.push_str(&safe);
            }
        }
    }
    message
}

#[cfg(test)]
#[path = "../tests/unit/diagnostic/tests.rs"]
mod tests;
