//! Selects the next resource grant needed by a provider.

use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ResourceAccess {
    pub alternatives_group: Option<String>,
    pub required: bool,
    pub granted: bool,
}

pub fn first_missing_resource(resources: &[ResourceAccess]) -> Option<usize> {
    let met_groups = resources
        .iter()
        .filter(|resource| resource.required && resource.granted)
        .filter_map(|resource| resource.alternatives_group.as_deref())
        .collect::<HashSet<_>>();
    resources.iter().position(|resource| {
        resource.required
            && !resource.granted
            && !resource
                .alternatives_group
                .as_deref()
                .is_some_and(|group| met_groups.contains(group))
    })
}

#[cfg(test)]
#[path = "../tests/unit/access/tests.rs"]
mod tests;
