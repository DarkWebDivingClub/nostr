// Copyright (c) 2021 Paul Miller
// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

use alloc::string::String;
use alloc::vec::Vec;

use aes::Aes256;
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::engine::{Engine, general_purpose};
use cbc::{Decryptor, Encryptor};
#[cfg(feature = "rand")]
use rand::Rng;
#[cfg(all(feature = "std", feature = "os-rng"))]
use rand::rand_core::UnwrapErr;
#[cfg(all(feature = "std", feature = "os-rng"))]
use rand::rngs::SysRng;

use crate::error::{Error, ErrorKind};
use crate::key::{PublicKey, SecretKey};
use crate::util;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

const IV_SIZE: usize = 16;
const ENCODED_IV_SIZE: usize = 24;

/// Encrypt
///
/// <div class="warning"><strong>Unsecure!</strong> Deprecated in favor of NIP17!</div>
#[inline]
#[cfg(all(feature = "std", feature = "os-rng"))]
pub fn encrypt<T>(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    content: T,
) -> Result<String, Error>
where
    T: AsRef<[u8]>,
{
    encrypt_with_rng(&mut UnwrapErr(SysRng), secret_key, public_key, content)
}

/// Encrypt
///
/// <div class="warning"><strong>Unsecure!</strong> Deprecated in favor of NIP17!</div>
#[cfg(feature = "rand")]
pub fn encrypt_with_rng<R, T>(
    rng: &mut R,
    secret_key: &SecretKey,
    public_key: &PublicKey,
    content: T,
) -> Result<String, Error>
where
    R: Rng,
    T: AsRef<[u8]>,
{
    // Generate iv
    let mut iv: [u8; IV_SIZE] = [0u8; IV_SIZE];
    rng.fill_bytes(&mut iv);

    encrypt_with_iv(secret_key, public_key, content, iv)
}

/// Encrypt
///
/// <div class="warning"><strong>Unsecure!</strong> Deprecated in favor of NIP17!</div>
pub fn encrypt_with_iv<T>(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    content: T,
    iv: [u8; IV_SIZE],
) -> Result<String, Error>
where
    T: AsRef<[u8]>,
{
    // Generate key
    let key: [u8; 32] = util::generate_shared_key(secret_key, public_key)?;

    // Compose cipher
    let cipher = Aes256CbcEnc::new(&key.into(), &iv.into());

    // Encrypt
    let result: Vec<u8> = cipher.encrypt_padded_vec_mut::<Pkcs7>(content.as_ref());

    // Encode with base64
    Ok(format!(
        "{}?iv={}",
        general_purpose::STANDARD.encode(result),
        general_purpose::STANDARD.encode(iv)
    ))
}

/// Decrypts content to bytes
///
/// <div class="warning"><strong>Unsecure!</strong> Deprecated in favor of NIP17!</div>
pub fn decrypt_to_bytes<S>(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    encrypted_content: S,
) -> Result<Vec<u8>, Error>
where
    S: Into<String>,
{
    let encrypted_content: String = encrypted_content.into();
    let Some((ciphertext, encoded_iv)) = encrypted_content.split_once("?iv=") else {
        return Err(Error::with_static_message(
            ErrorKind::Malformed,
            "invalid content format",
        ));
    };
    // Reject extra separators and oversized IVs before Base64 allocation.
    if encoded_iv.len() != ENCODED_IV_SIZE || encoded_iv.contains("?iv=") {
        return Err(Error::with_static_message(
            ErrorKind::Malformed,
            "invalid IV length",
        ));
    }

    let encrypted_content: Vec<u8> = general_purpose::STANDARD
        .decode(ciphertext)
        .map_err(Error::malformed_display)?;
    let iv: Vec<u8> = general_purpose::STANDARD
        .decode(encoded_iv)
        .map_err(Error::malformed_display)?;
    let iv: [u8; IV_SIZE] = iv
        .as_slice()
        .try_into()
        .map_err(|_| Error::with_static_message(ErrorKind::Malformed, "invalid IV length"))?;
    let key: [u8; 32] = util::generate_shared_key(secret_key, public_key)?;

    let cipher = Aes256CbcDec::new(&key.into(), &iv.into());
    let result = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(&encrypted_content)
        .map_err(|_| Error::with_static_message(ErrorKind::Crypto, "wrong block mode"))?;

    Ok(result)
}

/// Decrypts content to a UTF-8 string
///
/// <div class="warning"><strong>Unsecure!</strong> Deprecated in favor of NIP17!</div>
#[inline]
pub fn decrypt<T>(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    encrypted_content: T,
) -> Result<String, Error>
where
    T: Into<String>,
{
    let result = decrypt_to_bytes(secret_key, public_key, encrypted_content)?;
    String::from_utf8(result).map_err(Error::malformed)
}

#[cfg(all(test, feature = "std", feature = "os-rng"))]
mod tests {
    use core::str::FromStr;

    use super::*;
    use crate::key::Keys;

    #[test]
    fn test_encryption_decryption() {
        let sender_sk =
            SecretKey::from_str("6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
                .unwrap();
        let sender_keys = Keys::new(sender_sk);
        let sender_pk = sender_keys.public_key();

        let receiver_sk =
            SecretKey::from_str("7b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
                .unwrap();
        let receiver_keys = Keys::new(receiver_sk);
        let receiver_pk = receiver_keys.public_key();

        let encrypted_content_from_outside =
            "dJc+WbBgaFCD2/kfg1XCWJParplBDxnZIdJGZ6FCTOg=?iv=M6VxRPkMZu7aIdD+10xPuw==";

        let content = String::from("Saturn, bringer of old age");

        let encrypted_content = encrypt(sender_keys.secret_key(), &receiver_pk, &content).unwrap();

        assert_eq!(
            decrypt(receiver_keys.secret_key(), &sender_pk, encrypted_content).unwrap(),
            content
        );

        assert_eq!(
            decrypt(
                receiver_keys.secret_key(),
                &sender_pk,
                encrypted_content_from_outside
            )
            .unwrap(),
            content
        );

        assert_eq!(
            decrypt(
                sender_keys.secret_key(),
                &receiver_pk,
                "invalidcontentformat"
            )
            .unwrap_err()
            .kind(),
            ErrorKind::Malformed
        );
        assert_eq!(
            decrypt(
                sender_keys.secret_key(),
                &receiver_pk,
                "badbase64?iv=encode"
            )
            .unwrap_err()
            .kind(),
            ErrorKind::Malformed
        );

        // Content encrypted with aes256 using GCM mode
        assert_eq!(
            decrypt(
                sender_keys.secret_key(),
                &receiver_pk,
                "nseh0cQPEFID5C0CxYdcPwp091NhRQ==?iv=8PHy8/T19vf4+fr7/P3+/w=="
            )
            .unwrap_err()
            .kind(),
            ErrorKind::Crypto
        );
    }

    #[test]
    fn test_decryption_with_invalid_iv_length() {
        let sender_sk =
            SecretKey::from_str("6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
                .unwrap();
        let receiver_sk =
            SecretKey::from_str("7b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e")
                .unwrap();
        let receiver_pk = Keys::new(receiver_sk).public_key();

        for len in [0, 1, 15, 17, 32] {
            let ciphertext = general_purpose::STANDARD.encode([0u8; 16]);
            let iv = general_purpose::STANDARD.encode(vec![0u8; len]);
            let encrypted_content = format!("{ciphertext}?iv={iv}");

            let err = decrypt(&sender_sk, &receiver_pk, encrypted_content).unwrap_err();

            assert_eq!(err.kind(), ErrorKind::Malformed);
            assert_eq!(err.to_string(), "invalid IV length");
        }
    }

    #[test]
    fn test_decryption_rejects_multiple_iv_separators() {
        let keys = Keys::generate();
        let ciphertext = general_purpose::STANDARD.encode([0u8; 16]);
        let iv = general_purpose::STANDARD.encode([0u8; IV_SIZE]);
        let encrypted_content = format!("{ciphertext}?iv={iv}?iv={iv}");

        let err = decrypt(keys.secret_key(), &keys.public_key(), encrypted_content).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::Malformed);
        assert_eq!(err.to_string(), "invalid IV length");
    }
}
