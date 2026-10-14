//! Reads and writes generic passwords through the macOS Security framework.

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use objc2::rc::Retained;
use objc2_local_authentication::LAContext;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework_sys::base::{errSecItemNotFound, errSecParam, errSecSuccess};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecMatchLimit, kSecMatchLimitAll,
    kSecMatchSearchList, kSecReturnAttributes, kSecReturnData, kSecUseAuthenticationContext, kSecUseAuthenticationUI,
    kSecUseKeychain, kSecValueData,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemUpdate};

use super::services;

pub struct Item {
    pub data: Vec<u8>,
    pub account: Option<String>,
}

#[cfg_attr(test, mockall::automock)]
pub trait ItemCalls: Send + Sync {
    fn copy(&self, query: &CFDictionary<CFString, CFType>) -> Result<Option<CFType>, i32>;
    fn update(&self, query: &CFDictionary<CFString, CFType>, value: &CFDictionary<CFString, CFType>) -> i32;
    fn add(&self, query: &CFDictionary<CFString, CFType>) -> i32;
}

pub struct SystemCalls;

impl ItemCalls for SystemCalls {
    fn copy(&self, query: &CFDictionary<CFString, CFType>) -> Result<Option<CFType>, i32> {
        let mut value = std::ptr::null();
        let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut value) };
        if status == errSecItemNotFound {
            Ok(None)
        } else if status == errSecSuccess {
            Ok(Some(unsafe { CFType::wrap_under_create_rule(value) }))
        } else {
            Err(status)
        }
    }

    fn update(&self, query: &CFDictionary<CFString, CFType>, value: &CFDictionary<CFString, CFType>) -> i32 {
        unsafe { SecItemUpdate(query.as_concrete_TypeRef(), value.as_concrete_TypeRef()) }
    }

    fn add(&self, query: &CFDictionary<CFString, CFType>) -> i32 {
        unsafe { SecItemAdd(query.as_concrete_TypeRef(), std::ptr::null_mut()) }
    }
}

pub struct Store {
    calls: Box<dyn ItemCalls>,
    keychain: Option<SecKeychain>,
}

impl Store {
    pub fn system() -> Self {
        Self::with_calls(SystemCalls)
    }

    pub fn in_keychain(keychain: SecKeychain) -> Self {
        Self {
            calls: Box::new(SystemCalls),
            keychain: Some(keychain),
        }
    }

    pub fn with_calls(calls: impl ItemCalls + 'static) -> Self {
        Self {
            calls: Box::new(calls),
            keychain: None,
        }
    }

    pub fn load(&self, service: &str, account: Option<&str>) -> Result<Option<Item>, i32> {
        let Some(value) = self
            .calls
            .copy(&read_query(self.keychain.as_ref(), Some(service), account, true))?
        else {
            return Ok(None);
        };
        let values = value.downcast::<CFDictionary>().ok_or(errSecParam)?;
        let data = attribute(&values, unsafe { kSecValueData })
            .and_then(|value| value.downcast::<CFData>())
            .ok_or(errSecParam)?;
        let account = attribute(&values, unsafe { kSecAttrAccount })
            .and_then(|value| value.downcast::<CFString>())
            .map(|value| value.to_string());
        Ok(Some(Item {
            data: data.bytes().to_vec(),
            account,
        }))
    }

    pub fn service_names(&self, prefix: &str) -> Result<Vec<String>, i32> {
        let Some(value) = self
            .calls
            .copy(&read_query(self.keychain.as_ref(), None, None, false))?
        else {
            return Ok(Vec::new());
        };
        let values = value.downcast::<CFArray>().ok_or(errSecParam)?;
        let names = values
            .get_all_values()
            .into_iter()
            .map(|value| {
                let values = unsafe { CFType::wrap_under_get_rule(value.cast()) }
                    .downcast::<CFDictionary>()
                    .ok_or(errSecParam)?;
                Ok(attribute(&values, unsafe { kSecAttrService })
                    .and_then(|value| value.downcast::<CFString>())
                    .map(|value| value.to_string()))
            })
            .collect::<Result<Vec<Option<String>>, i32>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Ok(services(prefix, &names))
    }

    pub fn save(&self, data: &[u8], service: &str, account: &str) -> Result<(), i32> {
        let mut values = vec![
            (
                key(unsafe { kSecClass }),
                key(unsafe { kSecClassGenericPassword }).into_CFType(),
            ),
            (key(unsafe { kSecAttrService }), CFString::from(service).into_CFType()),
            (key(unsafe { kSecAttrAccount }), CFString::from(account).into_CFType()),
        ];
        if let Some(keychain) = &self.keychain {
            values.push((
                key(unsafe { kSecMatchSearchList }),
                CFArray::from_CFTypes(std::slice::from_ref(keychain)).into_CFType(),
            ));
        }
        let query = CFDictionary::from_CFType_pairs(&values);
        let update = CFDictionary::from_CFType_pairs(&[(
            key(unsafe { kSecValueData }),
            CFData::from_buffer(data).into_CFType(),
        )]);
        let status = self.calls.update(&query, &update);
        let status = if status == errSecItemNotFound {
            if let Some(keychain) = &self.keychain {
                values.retain(|(field, _)| *field != key(unsafe { kSecMatchSearchList }));
                values.push((key(unsafe { kSecUseKeychain }), keychain.as_CFType()));
            }
            values.push((key(unsafe { kSecValueData }), CFData::from_buffer(data).into_CFType()));
            let add = CFDictionary::from_CFType_pairs(&values);
            self.calls.add(&add)
        } else {
            status
        };
        if status == errSecSuccess { Ok(()) } else { Err(status) }
    }
}

fn key(value: CFStringRef) -> CFString {
    unsafe { CFString::wrap_under_get_rule(value) }
}

fn noninteractive_context() -> Retained<LAContext> {
    let context = unsafe { LAContext::new() };
    unsafe { context.setInteractionNotAllowed(true) };
    context
}

fn read_query(
    keychain: Option<&SecKeychain>,
    service: Option<&str>,
    account: Option<&str>,
    include_data: bool,
) -> CFDictionary<CFString, CFType> {
    let context = noninteractive_context();
    let mut values = vec![
        (
            key(unsafe { kSecClass }),
            key(unsafe { kSecClassGenericPassword }).into_CFType(),
        ),
        (
            key(unsafe { kSecReturnAttributes }),
            CFBoolean::true_value().into_CFType(),
        ),
        (key(unsafe { kSecUseAuthenticationContext }), unsafe {
            CFType::wrap_under_get_rule(Retained::as_ptr(&context).cast())
        }),
        (
            key(unsafe { kSecUseAuthenticationUI }),
            CFString::from("u_AuthUIF").into_CFType(),
        ),
    ];
    if let Some(service) = service {
        values.push((key(unsafe { kSecAttrService }), CFString::from(service).into_CFType()));
    }
    if let Some(keychain) = keychain {
        values.push((
            key(unsafe { kSecMatchSearchList }),
            CFArray::from_CFTypes(std::slice::from_ref(keychain)).into_CFType(),
        ));
    }
    if let Some(account) = account {
        values.push((key(unsafe { kSecAttrAccount }), CFString::from(account).into_CFType()));
    }
    if include_data {
        values.push((key(unsafe { kSecReturnData }), CFBoolean::true_value().into_CFType()));
    } else {
        values.push((
            key(unsafe { kSecMatchLimit }),
            key(unsafe { kSecMatchLimitAll }).into_CFType(),
        ));
    }
    CFDictionary::from_CFType_pairs(&values)
}

fn attribute(values: &CFDictionary, field: CFStringRef) -> Option<CFType> {
    let value = values.find(key(field).as_CFTypeRef().cast())?;
    Some(unsafe { CFType::wrap_under_get_rule((*value).cast()) })
}

#[cfg(test)]
#[path = "../../tests/unit/keychain/native.rs"]
mod tests;
