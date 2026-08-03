// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

//! HKDF Util

use bitcoin_hashes::hmac::{Hmac, HmacEngine};
use bitcoin_hashes::sha256::{self, Hash as Sha256Hash};
use bitcoin_hashes::{Hash, HashEngine};

/// HKDF extract
#[inline]
pub fn extract(salt: &[u8], input_key_material: &[u8]) -> Hmac<Sha256Hash> {
    let mut engine: HmacEngine<sha256::HashEngine> = HmacEngine::new(salt);
    engine.input(input_key_material);
    engine.finalize()
}

/// HKDF expand, filling `out` with `out.len()` bytes of output key material.
///
/// The caller owns the destination, so a fixed-size output needs no allocation.
///
/// Per RFC 5869, `out` may span at most 255 blocks (8160 bytes).
pub fn expand_into(prk: &[u8], info: &[u8], out: &mut [u8]) {
    debug_assert!(
        out.len() <= 255 * 32,
        "HKDF-expand supports at most 255 blocks of output"
    );

    // The keyed engine depends only on `prk`, and absorbing the ipad/opad key
    // schedule is most of the cost of a short HMAC. Build it once and clone the
    // midstate per block instead of re-keying for each block.
    let keyed: HmacEngine<sha256::HashEngine> = HmacEngine::new(prk);

    let mut prev: [u8; 32] = [0u8; 32];
    let mut written: usize = 0;
    let mut counter: u8 = 1;

    while written < out.len() {
        // T(i) = HMAC(PRK, T(i-1) | info | i), where T(0) is empty. Feeding the
        // parts in turn is byte-identical to feeding them joined.
        let mut engine: HmacEngine<sha256::HashEngine> = keyed.clone();
        if written > 0 {
            engine.input(&prev);
        }
        engine.input(info);
        engine.input(&[counter]);
        prev = engine.finalize().to_byte_array();

        let take: usize = (out.len() - written).min(prev.len());
        out[written..written + take].copy_from_slice(&prev[..take]);
        written += take;
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    struct Vector {
        ikm: &'static str,
        salt: &'static str,
        info: &'static str,
        len: usize,
        prk: &'static str,
        okm: &'static str,
    }

    /// RFC 5869 appendix A, test cases 1 to 3 (SHA-256).
    ///
    /// Case 2 covers a salt longer than the hash block size and an output
    /// spanning three blocks; case 3 covers an empty salt and empty info.
    const RFC5869: [Vector; 3] = [
        Vector {
            ikm: "0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
            salt: "000102030405060708090a0b0c",
            info: "f0f1f2f3f4f5f6f7f8f9",
            len: 42,
            prk: "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
            okm: "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865",
        },
        Vector {
            ikm: "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f",
            salt: "606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf",
            info: "b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff",
            len: 82,
            prk: "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244",
            okm: "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87",
        },
        Vector {
            ikm: "0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
            salt: "",
            info: "",
            len: 42,
            prk: "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
            okm: "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8",
        },
    ];

    #[test]
    fn test_rfc5869_vectors() {
        for (i, v) in RFC5869.iter().enumerate() {
            let prk = extract(&unhex(v.salt), &unhex(v.ikm));
            assert_eq!(
                prk.as_byte_array().as_slice(),
                unhex(v.prk),
                "case {} PRK",
                i + 1
            );

            let mut okm = vec![0u8; v.len];
            expand_into(prk.as_byte_array(), &unhex(v.info), &mut okm);
            assert_eq!(okm, unhex(v.okm), "case {} OKM", i + 1);
        }
    }

    /// Output stopping mid-block must be a prefix of the longer output.
    #[test]
    fn test_expand_partial_block() {
        let prk = extract(b"salt", b"ikm");
        let mut full = [0u8; 64];
        expand_into(prk.as_byte_array(), b"info", &mut full);

        for len in [1usize, 31, 32, 33, 63, 64] {
            let mut out = vec![0u8; len];
            expand_into(prk.as_byte_array(), b"info", &mut out);
            assert_eq!(out, full[..len], "length {len}");
        }
    }

    /// An empty destination must not invoke the PRF.
    #[test]
    fn test_expand_empty_output() {
        let prk = extract(b"salt", b"ikm");
        let mut out: [u8; 0] = [];
        expand_into(prk.as_byte_array(), b"info", &mut out);
    }
}
