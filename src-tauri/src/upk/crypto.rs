//! AES-256-ECB over the header tables (name / import / export / depends).
//!
//! ECB is deterministic block by block: re-encrypting an unchanged plaintext
//! yields identical bytes, so only the 16-byte blocks we touch change on disk.
//! Some packages (e.g. `TAGame.upk`) are not encrypted at all; [`open_header`]
//! detects that and returns the plaintext as-is.

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;

use crate::upk::error::{UpkError, UpkResult};
use crate::upk::keys::{AesKey, KeyRing};
use crate::upk::names;
use crate::upk::summary::PackageSummary;

const AES_BLOCK: usize = 16;

pub fn ecb_decrypt(ciphertext: &[u8], key: &AesKey) -> Vec<u8> {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut out = ciphertext.to_vec();
    for chunk in out.chunks_exact_mut(AES_BLOCK) {
        cipher.decrypt_block(GenericArray::from_mut_slice(chunk));
    }
    out
}

pub fn ecb_encrypt(plaintext: &[u8], key: &AesKey) -> Vec<u8> {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut out = plaintext.to_vec();
    for chunk in out.chunks_exact_mut(AES_BLOCK) {
        cipher.encrypt_block(GenericArray::from_mut_slice(chunk));
    }
    out
}

/// Decrypted header tables of one package.
#[derive(Debug, Clone)]
pub struct HeaderRegion {
    /// `None` when the package is not encrypted.
    pub key: Option<AesKey>,
    /// File offset of the region (= `name_offset`).
    pub start: usize,
    /// Plaintext of `[start, start + len)`.
    pub plain: Vec<u8>,
}

impl HeaderRegion {
    /// Offset of `file_offset` inside `plain`.
    pub fn rel(&self, file_offset: usize) -> Option<usize> {
        file_offset
            .checked_sub(self.start)
            .filter(|&r| r <= self.plain.len())
    }

    /// Writes the (re-encrypted) region back into a copy of the package.
    pub fn splice_into(&self, package: &mut [u8]) -> UpkResult<()> {
        let bytes = match &self.key {
            Some(key) => ecb_encrypt(&self.plain, key),
            None => self.plain.clone(),
        };
        let end = self.start + bytes.len();
        package
            .get_mut(self.start..end)
            .ok_or(UpkError::Truncated("header region"))?
            .copy_from_slice(&bytes);
        Ok(())
    }
}

/// A plausible name table starts with a sane length prefix followed by
/// printable ASCII, and its first `name_count` entries all parse.
fn looks_like_name_table(plain: &[u8], name_count: usize) -> bool {
    let Some(first_len) = crate::upk::reader::read_i32(plain, 0) else {
        return false;
    };
    if !(1..=1024).contains(&first_len) {
        return false;
    }
    let n = first_len as usize;
    let Some(payload) = plain.get(4..4 + n) else {
        return false;
    };
    if !payload.iter().all(|&b| b == 0 || (32..127).contains(&b)) {
        return false;
    }
    names::parse(plain, name_count.min(64)).len() == name_count.min(64)
}

/// Decrypts the header region, probing every key of the ring. Plain
/// packages are detected first and need no key.
pub fn open_header(
    buf: &[u8],
    summary: &PackageSummary,
    ring: &KeyRing,
) -> UpkResult<HeaderRegion> {
    let start = summary.name_offset;
    let len = summary.encrypted_region_len();
    let region = buf
        .get(start..start + len)
        .ok_or(UpkError::Truncated("header region"))?;

    if looks_like_name_table(region, summary.name_count) {
        return Ok(HeaderRegion {
            key: None,
            start,
            plain: region.to_vec(),
        });
    }
    if ring.is_empty() {
        return Err(UpkError::KeysMissing);
    }
    // Probe with the first two blocks only, then decrypt fully with the winner.
    let probe_len = len.min(4096) & !15;
    for key in ring.keys() {
        let probe = ecb_decrypt(&region[..probe_len], key);
        if looks_like_name_table(&probe, summary.name_count.min(8)) {
            return Ok(HeaderRegion {
                key: Some(*key),
                start,
                plain: ecb_decrypt(region, key),
            });
        }
    }
    Err(UpkError::NoKeyMatched)
}

/// Cheap check used by the catalog: can this package's header be read?
pub fn can_open(buf: &[u8], ring: &KeyRing) -> bool {
    PackageSummary::parse(buf)
        .and_then(|s| open_header(buf, &s, ring))
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecb_roundtrip() {
        let key = [3u8; 32];
        let plain: Vec<u8> = (0..64u8).collect();
        let enc = ecb_encrypt(&plain, &key);
        assert_ne!(enc, plain);
        assert_eq!(ecb_decrypt(&enc, &key), plain);
    }

    fn fake_names() -> Vec<u8> {
        let mut p = Vec::new();
        for name in ["Core", "Engine", "None", "Texture2D"] {
            p.extend(((name.len() + 1) as i32).to_le_bytes());
            p.extend(name.as_bytes());
            p.push(0);
            p.extend([0u8; 8]);
        }
        p.resize(p.len().next_multiple_of(16) + 16, 0);
        p
    }

    #[test]
    fn detects_plain_and_encrypted_headers() {
        let names = fake_names();
        let mut file = crate::upk::summary::tests::synthetic_summary(4, 64, 0);
        file.resize(64, 0);
        let total = 64 + names.len();
        file[8..12].copy_from_slice(&(total as u32).to_le_bytes());
        let key = [9u8; 32];

        let mut plain_file = file.clone();
        plain_file.extend(&names);
        let s = PackageSummary::parse(&plain_file).unwrap();
        let h = open_header(&plain_file, &s, &KeyRing::default()).unwrap();
        assert!(h.key.is_none());

        let mut enc_file = file;
        enc_file.extend(ecb_encrypt(&names, &key));
        use base64::Engine as _;
        let ring = KeyRing::from_text(&base64::prelude::BASE64_STANDARD.encode(key), None);
        let h = open_header(&enc_file, &s, &ring).unwrap();
        assert_eq!(h.key, Some(key));
        assert_eq!(&h.plain[..names.len()], &names[..]);
        assert!(matches!(
            open_header(&enc_file, &s, &KeyRing::default()),
            Err(UpkError::KeysMissing)
        ));
    }
}
