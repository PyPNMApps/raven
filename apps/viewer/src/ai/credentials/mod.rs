/*
 * Copyright 2026 PNM Viewer contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Explicit OS-vault persistence. No plaintext secret file or automatic provider calls.
mod record;
use super::Provider;
use crate::Diagnostic;
pub use record::{restore, snapshot, Credential};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, path::Path};
use zeroize::Zeroizing;

/// Application-owned vault namespace; provider IDs are fixed, never user paths.
#[cfg(not(test))]
const SERVICE: &str = "org.pypnmapps.raven.ai.v1";
#[cfg(test)]
const SERVICE: &str = "org.pypnmapps.raven.ai.v1.test";
/// Persistent secret boundary, injectable for deterministic failure tests.
pub trait Store {
    /// Reads one provider record without exposing it to presentation code.
    /// # Arguments
    /// * `provider` - Fixed supported provider.
    /// # Returns
    /// Zeroizing payload, or None for no saved entry.
    /// # Errors
    /// Sanitized locked/unavailable/access failures.
    fn read(&self, provider: Provider) -> Result<Option<Zeroizing<String>>, Diagnostic>;
    /// Writes one explicitly approved bounded record to the OS store.
    /// # Arguments
    /// * `provider`, `secret` - Fixed owner and private serialized credential.
    /// # Errors
    /// Fails without plaintext fallback when the vault refuses storage.
    fn write(&self, provider: Provider, secret: &str) -> Result<(), Diagnostic>;
    /// Removes one provider's saved record, accepting an already absent entry.
    /// # Arguments
    /// * `provider` - Selected fixed owner.
    /// # Errors
    /// Reports a vault failure; callers must not claim deletion succeeded.
    fn delete(&self, provider: Provider) -> Result<(), Diagnostic>;
}
/// Real platform store. Unsupported builds return an error, never a mock store.
pub struct OsStore;
fn unavailable() -> Diagnostic {
    tracing::warn!(target:"viewer::ai::credentials", code="AiVault", "OS credential vault operation failed");
    Diagnostic::new("AiVault", "The OS credential vault is locked, unavailable, or refused access. The operation could not be confirmed. No plaintext fallback was used. Unlock your keyring and retry explicitly.")
}
#[cfg(all(
    feature = "credential-store",
    any(target_os = "linux", target_os = "windows", target_os = "macos")
))]
fn entry(provider: Provider) -> Result<keyring::Entry, Diagnostic> {
    keyring::Entry::new(SERVICE, provider.id()).map_err(|_| unavailable())
}
impl Store for OsStore {
    /// Reads the selected platform vault entry with a bounded zeroizing result.
    ///
    /// Store failures return a sanitized diagnostic without secret or OS error text.
    fn read(&self, provider: Provider) -> Result<Option<Zeroizing<String>>, Diagnostic> {
        #[cfg(all(
            feature = "credential-store",
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        ))]
        {
            match entry(provider)?.get_password() {
                Ok(value) if value.len() <= 262_144 => Ok(Some(Zeroizing::new(value))),
                Ok(value) => {
                    drop(Zeroizing::new(value));
                    Err(unavailable())
                }
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(unavailable()),
            }
        }
        #[cfg(not(all(
            feature = "credential-store",
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        )))]
        {
            let _ = (provider, SERVICE);
            Err(unavailable())
        }
    }
    /// Writes only to the native platform store without a file fallback.
    ///
    /// Store failures return a sanitized diagnostic without secret or OS error text.
    fn write(&self, provider: Provider, secret: &str) -> Result<(), Diagnostic> {
        if secret.len() > 262_144 {
            return Err(unavailable());
        }
        #[cfg(all(
            feature = "credential-store",
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        ))]
        {
            entry(provider)?
                .set_password(secret)
                .map_err(|_| unavailable())
        }
        #[cfg(not(all(
            feature = "credential-store",
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        )))]
        {
            let _ = (provider, SERVICE);
            Err(unavailable())
        }
    }
    /// Deletes only the fixed provider entry; absence counts as already removed.
    ///
    /// Store failures return a sanitized diagnostic without secret or OS error text.
    fn delete(&self, provider: Provider) -> Result<(), Diagnostic> {
        #[cfg(all(
            feature = "credential-store",
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        ))]
        {
            match entry(provider)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => Err(unavailable()),
            }
        }
        #[cfg(not(all(
            feature = "credential-store",
            any(target_os = "linux", target_os = "windows", target_os = "macos")
        )))]
        {
            let _ = (provider, SERVICE);
            Err(unavailable())
        }
    }
}
/// Nonsecret saved-provider hints. Startup reads this file, never the vault.
#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub providers: BTreeMap<String, String>,
    pub preferred: Option<String>,
}
impl Index {
    /// Reads a bounded local nonsecret index without unlocking the credential vault.
    /// # Arguments
    /// * `path` - App-owned preferences file, never provider supplied.
    /// # Returns
    /// Empty index if absent, otherwise validated provider/model hints.
    /// # Errors
    /// Refuses malformed, oversized, redirected or unreadable preferences.
    pub fn read(path: &Path) -> Result<Self, Diagnostic> {
        let invalid = || {
            Diagnostic::new(
                "AiVaultIndex",
                "Saved connection preferences could not be read. No credentials were loaded.",
            )
        };
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            _ => return Err(invalid()),
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| invalid())?
            .take(8193)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if bytes.len() > 8192 {
            return Err(invalid());
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        if !value.valid() {
            return Err(invalid());
        }
        Ok(value)
    }
    fn valid(&self) -> bool {
        self.providers.len() <= 5
            && self
                .providers
                .iter()
                .all(|(id, model)| Provider::from_id(id).is_some() && record::model_id(model))
            && self
                .preferred
                .as_ref()
                .is_none_or(|id| self.providers.contains_key(id))
    }
    /// Atomically writes provider/model hints only, never any key or token.
    /// # Arguments
    /// * `path` - App-owned preferences location.
    /// # Errors
    /// Reports write/rename failures without claiming the vault entry was removed.
    pub fn write(&self, path: &Path) -> Result<(), Diagnostic> {
        let invalid = || {
            Diagnostic::new("AiVaultIndex","Saved connection preferences could not be updated. Check folder access and retry; credential storage or deletion may be incomplete.")
        };
        if !self.valid() || path.is_symlink() {
            return Err(invalid());
        }
        let directory = path.parent().ok_or_else(invalid)?;
        std::fs::create_dir_all(directory).map_err(|_| invalid())?;
        let mut output = tempfile::NamedTempFile::new_in(directory).map_err(|_| invalid())?;
        serde_json::to_writer(output.as_file_mut(), self).map_err(|_| invalid())?;
        output.persist(path).map_err(|_| invalid())?;
        Ok(())
    }
}

#[cfg(all(test, feature = "credential-store", target_os = "linux"))]
mod native_tests {
    use super::*;
    #[test]
    #[ignore = "Run explicitly under an isolated Secret Service bus and data directory"]
    fn isolated_secret_service_roundtrip() {
        assert_eq!(
            std::env::var("RAVEN_VAULT_TEST_ISOLATED").as_deref(),
            Ok("1")
        );
        let data = std::env::var_os("XDG_DATA_HOME").expect("private test data directory");
        assert!(Path::new(&data).join("raven-vault-test-marker").is_file());
        OsStore
            .write(Provider::Grok, "synthetic-native-vault-test")
            .unwrap();
        assert_eq!(
            OsStore.read(Provider::Grok).unwrap().unwrap().as_str(),
            "synthetic-native-vault-test"
        );
        OsStore.delete(Provider::Grok).unwrap();
        assert!(OsStore.read(Provider::Grok).unwrap().is_none());
    }
}
