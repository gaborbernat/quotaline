//! Parses status templates for native renderers.

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TemplateToken {
    Text(String),
    Placeholder(String),
    Newline,
}

pub fn parse_template(template: &str) -> Vec<TemplateToken> {
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\\' => {
                if let Some(next) = chars.next() {
                    if next == 'n' {
                        flush(&mut tokens, &mut text);
                        tokens.push(TemplateToken::Newline);
                    } else {
                        text.push(next);
                    }
                }
            }
            '\n' => {
                flush(&mut tokens, &mut text);
                tokens.push(TemplateToken::Newline);
            }
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                text.push('{');
            }
            '{' => {
                if let Some(first) = chars.next() {
                    let mut name = first.to_string();
                    let mut closed = false;
                    for inner in chars.by_ref() {
                        if inner == '}' {
                            closed = true;
                            break;
                        }
                        name.push(inner);
                    }
                    if closed {
                        flush(&mut tokens, &mut text);
                        tokens.push(TemplateToken::Placeholder(name));
                    } else {
                        text.push('{');
                        text.push_str(&name);
                    }
                } else {
                    text.push('{');
                }
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                }
                text.push('}');
            }
            _ => text.push(character),
        }
    }
    flush(&mut tokens, &mut text);
    tokens
}

fn flush(tokens: &mut Vec<TemplateToken>, text: &mut String) {
    if !text.is_empty() {
        tokens.push(TemplateToken::Text(std::mem::take(text)));
    }
}

#[cfg(test)]
#[path = "../tests/unit/template/tests.rs"]
mod tests;
