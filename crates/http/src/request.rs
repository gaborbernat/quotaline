//! Applies shared request origin and endpoint logging policy.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Origin<'a> {
    pub scheme: Option<&'a str>,
    pub host: Option<&'a str>,
    pub port: Option<i32>,
}

pub fn same_origin(source: Origin<'_>, destination: Origin<'_>) -> bool {
    equal_part(source.scheme, destination.scheme)
        && equal_part(source.host, destination.host)
        && source.port == destination.port
}

pub fn redacted_endpoint(value: &str) -> String {
    value
        .split('/')
        .map(|component| if is_identifier(component) { "{id}" } else { component })
        .collect::<Vec<_>>()
        .join("/")
}

fn equal_part(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.eq_ignore_ascii_case(right),
        (None, None) => true,
        _ => false,
    }
}

fn is_identifier(component: &str) -> bool {
    component.split('-').map(str::len).eq([8, 4, 4, 4, 12])
        && component.bytes().all(|byte| byte == b'-' || byte.is_ascii_hexdigit())
}

#[cfg(test)]
#[path = "../tests/unit/request/tests.rs"]
mod tests;
