use alloc::string::String;
use core::fmt;
use core::str::{self, FromStr};

use serde::{Deserialize, Deserializer, Serialize};

use crate::error::{Error, ErrorKind};
use crate::util;

const LEN: usize = 64;

/// Event signature
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signature(secp256k1::schnorr::Signature);

impl Signature {
    /// Construct from 64-byte array
    #[inline]
    pub fn from_byte_array(bytes: [u8; LEN]) -> Self {
        Self(secp256k1::schnorr::Signature::from_byte_array(bytes))
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self, Error> {
        let bytes: [u8; LEN] = util::hex_decode(hex)?;
        Ok(Self::from_byte_array(bytes))
    }

    /// Parse from bytes
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        // Check len
        if slice.len() != LEN {
            return Err(Error::with_static_message(
                ErrorKind::Invalid,
                "invalid signature",
            ));
        }

        // Copy bytes
        let mut bytes: [u8; LEN] = [0u8; LEN];
        bytes.copy_from_slice(slice);

        // Construct
        Ok(Self::from_byte_array(bytes))
    }

    /// Get as bytes
    #[inline]
    pub fn as_bytes(&self) -> &[u8; LEN] {
        self.0.as_byte_array()
    }

    /// Consume and get bytes
    #[inline]
    pub fn to_bytes(self) -> [u8; LEN] {
        self.0.to_byte_array()
    }

    /// Get as hex string
    #[inline]
    pub fn to_hex(&self) -> String {
        // SAFETY: hex is a valid UTF-8
        unsafe { String::from_utf8_unchecked(self.hex_byte_array().to_vec()) }
    }

    #[inline]
    fn hex_byte_array(&self) -> [u8; LEN * 2] {
        let mut buf = [0u8; LEN * 2];
        faster_hex::hex_encode(self.as_bytes(), &mut buf).expect("Buffer size is correct");
        buf
    }
}

impl Signature {
    #[inline]
    pub(crate) fn from_secp256k1(sig: secp256k1::schnorr::Signature) -> Self {
        Self(sig)
    }

    #[inline]
    pub(crate) fn as_secp256k1(&self) -> &secp256k1::schnorr::Signature {
        &self.0
    }
}

impl FromStr for Signature {
    type Err = Error;

    /// Parse the signature from hex string
    fn from_str(id: &str) -> Result<Self, Self::Err> {
        Self::from_hex(id)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl AsRef<[u8; LEN]> for Signature {
    fn as_ref(&self) -> &[u8; LEN] {
        self.as_bytes()
    }
}

impl fmt::LowerHex for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(self, f)
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes: [u8; LEN * 2] = self.hex_byte_array();
        // SAFETY: hex is a valid UTF-8
        let encoded: &str = unsafe { str::from_utf8_unchecked(&bytes) };
        serializer.serialize_str(encoded)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let id: String = String::deserialize(deserializer)?;
        Self::from_hex(&id).map_err(serde::de::Error::custom)
    }
}
