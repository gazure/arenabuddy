use keyring::Entry;
use zeroize::Zeroizing;

const SERVICE: &str = "com.arenabuddy.typesafe";
const ACCOUNT: &str = "api-key";

/// A credential failure that contains no secret or platform error payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CredentialError {
    #[error("Enter a nonempty API key without spaces or control characters.")]
    InvalidKey,
    #[error("Secure storage is unavailable or access was denied. Unlock your system credential store and try again.")]
    Unavailable,
}

/// Loads the key from the OS credential store, returning `None` if no key exists.
///
/// Call this blocking function on a worker thread. Store failures remain errors.
pub(crate) fn load_typesafe_key() -> Result<Option<Zeroizing<String>>, CredentialError> {
    let entry = Entry::new(SERVICE, ACCOUNT).map_err(|_| CredentialError::Unavailable)?;
    read_key(entry.get_password())
}

fn read_key(result: keyring::Result<String>) -> Result<Option<Zeroizing<String>>, CredentialError> {
    match result {
        Ok(key) => Ok(Some(Zeroizing::new(key))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(CredentialError::Unavailable),
    }
}

/// Saves or replaces `key` in OS storage after validating its local format.
///
/// Call this blocking function on a worker thread. Returns an error for invalid
/// input or unavailable storage. This does not validate access to the API.
pub(crate) fn save_typesafe_key(key: &str) -> Result<(), CredentialError> {
    let key = validated_key(key)?;
    Entry::new(SERVICE, ACCOUNT)
        .and_then(|entry| entry.set_password(key))
        .map_err(|_| CredentialError::Unavailable)
}

fn validated_key(key: &str) -> Result<&str, CredentialError> {
    let key = key.trim();
    if key.is_empty() || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Err(CredentialError::InvalidKey)
    } else {
        Ok(key)
    }
}

/// Removes the saved key, succeeding if it is already absent.
///
/// Call this blocking function on a worker thread. Returns an error if storage
/// cannot be accessed. No local-file fallback is used.
pub(crate) fn remove_typesafe_key() -> Result<(), CredentialError> {
    let result = Entry::new(SERVICE, ACCOUNT).and_then(|entry| entry.delete_credential());
    match result {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(CredentialError::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_without_assuming_a_provider_key_prefix() {
        assert_eq!(validated_key("  example-key\n"), Ok("example-key"));
        for invalid in ["", " \n", "two words", "key\nvalue", "key\0value"] {
            assert_eq!(validated_key(invalid), Err(CredentialError::InvalidKey));
        }
    }

    #[test]
    fn missing_key_is_distinct_from_storage_failure() {
        assert_eq!(read_key(Err(keyring::Error::NoEntry)), Ok(None));
        assert_eq!(
            read_key(Err(keyring::Error::NoDefaultStore)),
            Err(CredentialError::Unavailable)
        );
    }

    #[test]
    fn errors_do_not_expose_platform_payloads() {
        let error = read_key(Err(keyring::Error::BadEncoding(b"secret-key".to_vec()))).unwrap_err();
        assert!(!format!("{error} {error:?}").contains("secret-key"));
    }
}
