//! Secret storage only: no credential is serializable or printable.
use crate::error::{AppError, AppResult};
use ring::{
    aead, hmac,
    rand::{SecureRandom, SystemRandom},
};
use secrecy::{ExposeSecret, SecretString};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path},
};
use uuid::Uuid;

pub struct CredentialCipher {
    key: aead::LessSafeKey,
    commitment_key: hmac::Key,
}

impl CredentialCipher {
    /// The environment format is exactly 64 hexadecimal characters (32 bytes).
    pub fn from_hex(value: &SecretString) -> AppResult<Self> {
        let value = value.expose_secret();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(storage_error());
        }
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| storage_error())?;
        }
        let result = Self::from_bytes(&bytes);
        bytes.fill(0);
        result
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> AppResult<Self> {
        let key = aead::UnboundKey::new(&aead::AES_256_GCM, bytes).map_err(|_| storage_error())?;
        let derivation = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, bytes),
            b"ai-center-credential-request-commitment-v1",
        );
        Ok(Self {
            key: aead::LessSafeKey::new(key),
            commitment_key: hmac::Key::new(hmac::HMAC_SHA256, derivation.as_ref()),
        })
    }

    /// A private, non-symlink local directory is required. Never follows an
    /// existing key symlink; never overwrites a key or repairs loose permissions.
    #[cfg(unix)]
    pub fn local(directory: &Path) -> AppResult<Self> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
        if !directory.is_absolute() {
            return Err(storage_error());
        }
        let mut current = std::path::PathBuf::new();
        for part in directory.components() {
            match part {
                Component::RootDir | Component::Normal(_) => current.push(part.as_os_str()),
                _ => return Err(storage_error()),
            }
            if !current.exists() {
                fs::DirBuilder::new()
                    .mode(0o700)
                    .create(&current)
                    .map_err(|_| storage_error())?;
            }
            let metadata = fs::symlink_metadata(&current).map_err(|_| storage_error())?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(storage_error());
            }
        }
        let directory_meta = fs::symlink_metadata(directory).map_err(|_| storage_error())?;
        if directory_meta.permissions().mode() & 0o777 != 0o700 {
            return Err(storage_error());
        }
        let path = directory.join("credentials.key");
        let mut bytes = [0_u8; 32];
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
            .open(&path)
        {
            Ok(mut file) => {
                SystemRandom::new()
                    .fill(&mut bytes)
                    .map_err(|_| storage_error())?;
                file.write_all(&bytes).map_err(|_| storage_error())?;
                file.sync_all().map_err(|_| storage_error())?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&path).map_err(|_| storage_error())?;
                if !metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || metadata.permissions().mode() & 0o777 != 0o600
                    || metadata.nlink() != 1
                    || metadata.uid() != directory_meta.uid()
                {
                    return Err(storage_error());
                }
                let mut file = fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(nix::fcntl::OFlag::O_NOFOLLOW.bits())
                    .open(&path)
                    .map_err(|_| storage_error())?;
                let opened = file.metadata().map_err(|_| storage_error())?;
                if opened.ino() != metadata.ino()
                    || opened.dev() != metadata.dev()
                    || opened.len() != 32
                {
                    return Err(storage_error());
                }
                file.read_exact(&mut bytes).map_err(|_| storage_error())?;
            }
            Err(_) => return Err(storage_error()),
        }
        let result = Self::from_bytes(&bytes);
        bytes.fill(0);
        result
    }

    #[cfg(not(unix))]
    pub fn local(_directory: &Path) -> AppResult<Self> {
        Err(storage_error())
    }

    pub fn encrypt(
        &self,
        workspace: Uuid,
        actor: Uuid,
        id: Uuid,
        provider: &str,
        secret: &SecretString,
    ) -> AppResult<Vec<u8>> {
        let mut nonce = [0_u8; 12];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| storage_error())?;
        let mut encrypted = secret.expose_secret().as_bytes().to_vec();
        self.key
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad(workspace, actor, id, provider)),
                &mut encrypted,
            )
            .map_err(|_| storage_error())?;
        let mut envelope = vec![1_u8];
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&encrypted);
        Ok(envelope)
    }

    pub fn decrypt(
        &self,
        workspace: Uuid,
        actor: Uuid,
        id: Uuid,
        provider: &str,
        envelope: &[u8],
    ) -> AppResult<SecretString> {
        if envelope.len() < 30 || envelope[0] != 1 {
            return Err(storage_error());
        }
        let nonce: [u8; 12] = envelope[1..13].try_into().map_err(|_| storage_error())?;
        let mut encrypted = envelope[13..].to_vec();
        let plain = self
            .key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::from(aad(workspace, actor, id, provider)),
                &mut encrypted,
            )
            .map_err(|_| storage_error())?;
        let value = std::str::from_utf8(plain)
            .map_err(|_| storage_error())?
            .to_owned();
        encrypted.fill(0);
        Ok(SecretString::from(value))
    }

    #[must_use]
    pub fn commitment(&self, secret: &SecretString) -> String {
        use std::fmt::Write as _;
        hmac::sign(&self.commitment_key, secret.expose_secret().as_bytes())
            .as_ref()
            .iter()
            .fold(String::with_capacity(64), |mut output, byte| {
                let _ = write!(output, "{byte:02x}");
                output
            })
    }
}

fn aad(workspace: Uuid, actor: Uuid, id: Uuid, provider: &str) -> Vec<u8> {
    format!("ai-center-provider-key-v1\0{workspace}\0{actor}\0{id}\0{provider}").into_bytes()
}

pub(super) fn storage_error() -> AppError {
    AppError::Agent("Le stockage sécurisé des connexions IA est indisponible. Vérifiez la configuration du serveur.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cipher() -> CredentialCipher {
        let mut key = [0; 32];
        SystemRandom::new().fill(&mut key).unwrap();
        CredentialCipher::from_bytes(&key).unwrap()
    }
    #[test]
    fn envelopes_are_random_authenticated_and_bound_to_every_identity() {
        let cipher = cipher();
        let w = Uuid::new_v4();
        let a = Uuid::new_v4();
        let id = Uuid::new_v4();
        let secret = SecretString::from(Uuid::new_v4().to_string());
        let mut first = cipher.encrypt(w, a, id, "openai", &secret).unwrap();
        assert_ne!(first, cipher.encrypt(w, a, id, "openai", &secret).unwrap());
        assert!(
            !first
                .windows(secret.expose_secret().len())
                .any(|part| part == secret.expose_secret().as_bytes())
        );
        assert_eq!(
            cipher
                .decrypt(w, a, id, "openai", &first)
                .unwrap()
                .expose_secret(),
            secret.expose_secret()
        );
        for (ww, aa, ii, pp) in [
            (Uuid::new_v4(), a, id, "openai"),
            (w, Uuid::new_v4(), id, "openai"),
            (w, a, Uuid::new_v4(), "openai"),
            (w, a, id, "anthropic"),
        ] {
            assert!(cipher.decrypt(ww, aa, ii, pp, &first).is_err());
        }
        first[15] ^= 1;
        assert!(cipher.decrypt(w, a, id, "openai", &first).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn private_key_survives_restart_and_rejects_links_or_loose_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = std::env::temp_dir().join(format!("ai-center-cipher-{}", Uuid::new_v4()));
        let first = CredentialCipher::local(&root).unwrap();
        let secret = SecretString::from(Uuid::new_v4().to_string());
        assert_eq!(
            first.commitment(&secret),
            CredentialCipher::local(&root).unwrap().commitment(&secret)
        );
        let key = root.join("credentials.key");
        fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(CredentialCipher::local(&root).is_err());
        fs::remove_file(&key).unwrap();
        symlink(root.join("missing"), &key).unwrap();
        assert!(CredentialCipher::local(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
