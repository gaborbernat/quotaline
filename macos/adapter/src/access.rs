//! Exposes shared resource grant selection to Swift.

use quotaline_setup::access;
use quotaline_setup::access::ResourceAccess;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ResourceAccessRecord {
    pub alternatives_group: Option<String>,
    pub required: bool,
    pub granted: bool,
}

impl From<ResourceAccessRecord> for ResourceAccess {
    fn from(value: ResourceAccessRecord) -> Self {
        Self {
            alternatives_group: value.alternatives_group,
            required: value.required,
            granted: value.granted,
        }
    }
}

#[uniffi::export]
pub fn first_missing_resource(resources: Vec<ResourceAccessRecord>) -> Option<u32> {
    access::first_missing_resource(&resources.into_iter().map(ResourceAccess::from).collect::<Vec<_>>())
        .map(|index| index as u32)
}

#[cfg(test)]
#[path = "../tests/unit/access/tests.rs"]
mod tests;
