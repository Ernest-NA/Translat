use std::fs;
use std::path::Path;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;

use crate::persistence::error::PersistenceError;

pub fn load_or_create_encryption_key(key_path: &Path) -> Result<String, PersistenceError> {
    if key_path.exists() {
        return load_existing_encryption_key(key_path);
    }

    if let Some(parent_directory) = key_path.parent() {
        fs::create_dir_all(parent_directory).map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The persistence bootstrap could not create the key directory at {}.",
                    parent_directory.display()
                ),
                error,
            )
        })?;
    }

    let mut secret_bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut secret_bytes);

    let encryption_key = URL_SAFE_NO_PAD.encode(secret_bytes);
    let protected_key_bytes = protect_key(encryption_key.as_bytes())?;

    fs::write(key_path, protected_key_bytes).map_err(|error| {
        PersistenceError::with_details(
            format!(
                "The persistence bootstrap could not store the encrypted key at {}.",
                key_path.display()
            ),
            error,
        )
    })?;

    Ok(encryption_key)
}

pub fn load_existing_encryption_key(key_path: &Path) -> Result<String, PersistenceError> {
    let protected_key_bytes = fs::read(key_path).map_err(|error| {
        PersistenceError::with_details(
            format!(
                "The persistence bootstrap could not read the encrypted key at {}.",
                key_path.display()
            ),
            error,
        )
    })?;

    let decrypted_key_bytes = unprotect_key(&protected_key_bytes)?;

    String::from_utf8(decrypted_key_bytes).map_err(|error| {
        PersistenceError::with_details(
            "The persistence bootstrap could not decode the stored encryption key.",
            error,
        )
    })
}

#[cfg(target_os = "windows")]
fn protect_key(plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
    windows_dpapi::protect(plaintext)
}

#[cfg(not(target_os = "windows"))]
fn protect_key(_plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
    Err(PersistenceError::new(
        "The encrypted SQLite key bootstrap is currently supported only on Windows.",
    ))
}

#[cfg(target_os = "windows")]
fn unprotect_key(ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
    windows_dpapi::unprotect(ciphertext)
}

#[cfg(not(target_os = "windows"))]
fn unprotect_key(_ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
    Err(PersistenceError::new(
        "The encrypted SQLite key bootstrap is currently supported only on Windows.",
    ))
}

#[cfg(target_os = "windows")]
mod windows_dpapi {
    use std::io;
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    use crate::persistence::error::PersistenceError;

    pub fn protect(plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        let mut input_blob = blob_from_bytes(plaintext);
        let mut output_blob = empty_blob();
        let description = wide_description("Translat encrypted database key");

        let protected_ok = unsafe {
            CryptProtectData(
                &mut input_blob,
                description.as_ptr(),
                null(),
                null(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        };

        if protected_ok == 0 {
            return Err(PersistenceError::with_details(
                "The persistence bootstrap could not protect the database key with DPAPI.",
                io::Error::last_os_error(),
            ));
        }

        let protected_bytes = copy_blob_bytes(&output_blob);
        free_blob(&mut output_blob);

        Ok(protected_bytes)
    }

    pub fn unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        let mut input_blob = blob_from_bytes(ciphertext);
        let mut output_blob = empty_blob();

        let unprotected_ok = unsafe {
            CryptUnprotectData(
                &mut input_blob,
                null_mut(),
                null(),
                null(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output_blob,
            )
        };

        if unprotected_ok == 0 {
            return Err(PersistenceError::with_details(
                "The persistence bootstrap could not decrypt the stored database key with DPAPI.",
                io::Error::last_os_error(),
            ));
        }

        let decrypted_bytes = copy_blob_bytes(&output_blob);
        free_blob(&mut output_blob);

        Ok(decrypted_bytes)
    }

    fn blob_from_bytes(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
            pbData: bytes.as_ptr().cast_mut(),
        }
    }

    fn empty_blob() -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        }
    }

    fn copy_blob_bytes(blob: &CRYPT_INTEGER_BLOB) -> Vec<u8> {
        if blob.pbData.is_null() || blob.cbData == 0 {
            return Vec::new();
        }

        unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize) }.to_vec()
    }

    fn free_blob(blob: &mut CRYPT_INTEGER_BLOB) {
        if !blob.pbData.is_null() {
            unsafe {
                LocalFree(blob.pbData.cast());
            }
            blob.pbData = null_mut();
            blob.cbData = 0;
        }
    }

    fn wide_description(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}
