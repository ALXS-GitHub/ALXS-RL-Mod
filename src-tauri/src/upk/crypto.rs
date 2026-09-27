//! Package encryption.
//!
//! - Most packages: AES-256-ECB over the header tables (name / import /
//!   export / depends); the compressed body is plain. ECB is deterministic
//!   block by block, so only the 16-byte blocks we touch change on disk.
//! - "Fully encrypted" packages (licensee 33+, flag `0x0800`, new items since
//!   late August 2026): AES-256-CTR over the header tables *and* over every
//!   compressed chunk, each with its own 12-byte nonce (the header's ends the
//!   summary; the chunks' sit in the chunk table). The key is still one of
//!   `keys.txt`. CTR is a keystream XOR, so re-encrypting unchanged bytes
//!   with the same key and nonce gives the same bytes back.
//! - Some packages (e.g. `TAGame.upk`) are not encrypted at all;
//!   [`open_header`] detects that and returns the plaintext as-is.
//!
//! The game picks the key from the package *name*: a package written under
//! another name must be re-encrypted with that name's key.

use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;

use crate::upk::error::{UpkError, UpkResult};
use crate::upk::keys::{AesKey, KeyRing};
use crate::upk::names;
use crate::upk::summary::PackageSummary;

const AES_BLOCK: usize = 16;

pub type Nonce = [u8; 12];

/// AES-256-CTR (encryption == decryption): 12-byte nonce followed by a
/// 32-bit big-endian block counter starting at `counter`.
pub fn ctr_xor(data: &mut [u8], key: &AesKey, nonce: &Nonce, counter: u32) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = [0u8; AES_BLOCK];
    block[..12].copy_from_slice(nonce);
    for (i, chunk) in data.chunks_mut(AES_BLOCK).enumerate() {
        block[12..].copy_from_slice(&counter.wrapping_add(i as u32).to_be_bytes());
        let mut stream = GenericArray::clone_from_slice(&block);
        cipher.encrypt_block(&mut stream);
        for (b, k) in chunk.iter_mut().zip(stream.iter()) {
            *b ^= k;
        }
    }
}

/// One AES-CTR compressed chunk of a fully encrypted package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedChunk {
    pub offset: usize,
    pub len: usize,
    pub nonce: Nonce,
}

/// Size of an entry of Rocket League's compressed-chunk table:
/// `i64 u_off · i32 u_size · i64 c_off · i32 c_size · 12-byte nonce`.
const CHUNK_ENTRY: usize = 36;

/// Finds the chunk table of a fully encrypted package in its decrypted
/// header. Accepted only if the chunks follow each other from
/// `total_header_size` on, without gap. (The file may be read partially,
/// as the catalog does: [`xor_chunks`] checks the bounds.)
pub fn encrypted_chunks(plain: &[u8], total_header_size: usize) -> Option<Vec<EncryptedChunk>> {
    let first = (total_header_size as u64).to_le_bytes();
    let u64_at = |pos: usize| -> Option<u64> {
        Some(u64::from_le_bytes(
            plain.get(pos..pos + 8)?.try_into().ok()?,
        ))
    };
    let usize_at = |pos: usize| -> Option<usize> {
        crate::upk::reader::read_i32(plain, pos).and_then(|v| usize::try_from(v).ok())
    };
    for at in 4..plain.len().saturating_sub(CHUNK_ENTRY) {
        if plain.get(at + 12..at + 20) != Some(&first[..]) {
            continue;
        }
        let Some(count) = usize_at(at - 4).filter(|n| (1..=4096).contains(n)) else {
            continue;
        };
        let mut out = Vec::with_capacity(count);
        let mut expect = total_header_size;
        for k in 0..count {
            let e = at + k * CHUNK_ENTRY;
            let (Some(c_off), Some(c_size), Some(nonce)) = (
                u64_at(e + 12).and_then(|v| usize::try_from(v).ok()),
                usize_at(e + 20),
                plain.get(e + 24..e + CHUNK_ENTRY),
            ) else {
                break;
            };
            if c_off != expect || c_size == 0 {
                break;
            }
            let mut n = [0u8; 12];
            n.copy_from_slice(nonce);
            out.push(EncryptedChunk {
                offset: c_off,
                len: c_size,
                nonce: n,
            });
            expect += c_size;
        }
        if out.len() == count {
            return Some(out);
        }
    }
    None
}

/// XORs every chunk with its keystream: decrypts encrypted chunks, encrypts
/// plain ones.
pub fn xor_chunks(file: &mut [u8], chunks: &[EncryptedChunk], key: &AesKey) -> UpkResult<()> {
    for c in chunks {
        let data = file
            .get_mut(c.offset..c.offset + c.len)
            .ok_or(UpkError::Truncated("encrypted chunk"))?;
        ctr_xor(data, key, &c.nonce, 0);
    }
    Ok(())
}

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
    /// AES-CTR nonce of fully encrypted packages (`None` = ECB).
    pub nonce: Option<Nonce>,
    /// Compressed chunks of fully encrypted packages (empty otherwise).
    pub chunks: Vec<EncryptedChunk>,
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
        let bytes = match (&self.key, &self.nonce) {
            (Some(key), Some(nonce)) => {
                let mut b = self.plain.clone();
                ctr_xor(&mut b, key, nonce, 0);
                b
            }
            (Some(key), None) => ecb_encrypt(&self.plain, key),
            (None, _) => self.plain.clone(),
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

    if summary.header_nonce.is_none() && looks_like_name_table(region, summary.name_count) {
        return Ok(HeaderRegion {
            key: None,
            nonce: None,
            chunks: Vec::new(),
            start,
            plain: region.to_vec(),
        });
    }
    if ring.is_empty() {
        return Err(UpkError::KeysMissing);
    }
    // Probe with the first blocks only, then decrypt fully with the winner.
    let probe_len = len.min(4096) & !15;
    for key in ring.keys() {
        let Some(nonce) = summary.header_nonce else {
            let probe = ecb_decrypt(&region[..probe_len], key);
            if looks_like_name_table(&probe, summary.name_count.min(8)) {
                return Ok(HeaderRegion {
                    key: Some(*key),
                    nonce: None,
                    chunks: Vec::new(),
                    start,
                    plain: ecb_decrypt(region, key),
                });
            }
            continue;
        };
        let mut probe = region[..probe_len].to_vec();
        ctr_xor(&mut probe, key, &nonce, 0);
        if !looks_like_name_table(&probe, summary.name_count.min(8)) {
            continue;
        }
        let mut plain = region.to_vec();
        ctr_xor(&mut plain, key, &nonce, 0);
        let chunks = encrypted_chunks(&plain, summary.total_header_size)
            .ok_or_else(|| UpkError::Format("encrypted package without a chunk table".into()))?;
        return Ok(HeaderRegion {
            key: Some(*key),
            nonce: Some(nonce),
            chunks,
            start,
            plain,
        });
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
pub(crate) mod tests {
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
    fn ctr_roundtrip_and_counter_offset() {
        let key = [5u8; 32];
        let nonce = [7u8; 12];
        let plain: Vec<u8> = (0..100u8).collect();
        let mut enc = plain.clone();
        ctr_xor(&mut enc, &key, &nonce, 0);
        assert_ne!(enc, plain);
        // Decrypting from the second block on uses counter 1.
        let mut tail = enc[16..].to_vec();
        ctr_xor(&mut tail, &key, &nonce, 1);
        assert_eq!(tail, plain[16..]);
        ctr_xor(&mut enc, &key, &nonce, 0);
        assert_eq!(enc, plain);
    }

    /// A fully encrypted package (licensee 34, flag 0x0800): CTR header whose
    /// nonce ends the summary, and one CTR chunk listed in the chunk table.
    pub(crate) fn fully_encrypted(key: &AesKey, payload: &[u8]) -> Vec<u8> {
        const NAME_OFFSET: usize = 96;
        let header_nonce = [0x11u8; 12];
        let chunk_nonce = [0x22u8; 12];
        let chunk = crate::upk::chunks::wrap_chunked(payload).unwrap();

        let mut region = fake_names();
        region.extend(1i32.to_le_bytes());
        let table_len = region.len() + CHUNK_ENTRY + 16;
        let total = NAME_OFFSET + table_len.next_multiple_of(16);
        region.extend(0u64.to_le_bytes());
        region.extend((payload.len() as i32).to_le_bytes());
        region.extend((total as u64).to_le_bytes());
        region.extend((chunk.len() as i32).to_le_bytes());
        region.extend(chunk_nonce);
        region.resize(total - NAME_OFFSET, 0);
        ctr_xor(&mut region, key, &header_nonce, 0);

        let mut file = crate::upk::summary::tests::synthetic_summary(4, NAME_OFFSET as u32, 0);
        file[6..8].copy_from_slice(&34u16.to_le_bytes());
        file[8..12].copy_from_slice(&(total as u32).to_le_bytes());
        file[21..25].copy_from_slice(&crate::upk::summary::FULL_ENCRYPTED_FLAG.to_le_bytes());
        file.resize(NAME_OFFSET - 12, 0);
        file.extend(header_nonce);
        file.extend(region);
        let mut body = chunk;
        ctr_xor(&mut body, key, &chunk_nonce, 0);
        file.extend(body);
        file
    }

    #[test]
    fn opens_fully_encrypted_packages() {
        use base64::Engine as _;
        let key = [4u8; 32];
        let payload: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let file = fully_encrypted(&key, &payload);
        let s = PackageSummary::parse(&file).unwrap();
        assert_eq!(s.header_nonce, Some([0x11; 12]));

        let ring = KeyRing::from_text(&base64::prelude::BASE64_STANDARD.encode(key), None);
        let h = open_header(&file, &s, &ring).unwrap();
        assert_eq!(h.key, Some(key));
        assert_eq!(h.nonce, Some([0x11; 12]));
        assert_eq!(h.chunks.len(), 1);
        assert_eq!(h.chunks[0].offset, s.total_header_size);
        assert_eq!(h.chunks[0].offset + h.chunks[0].len, file.len());

        // A partial read (as the catalog does) still opens.
        assert!(can_open(&file[..s.total_header_size], &ring));
        let other = KeyRing::from_text(&base64::prelude::BASE64_STANDARD.encode([8u8; 32]), None);
        assert!(!can_open(&file, &other));

        // Opened, the body decompresses; header + seal give the file back.
        let pkg = crate::upk::Package::open(file.clone(), &ring).unwrap();
        let map = crate::upk::chunks::ChunkMap::read_from(&pkg.bytes, s.total_header_size).unwrap();
        assert_eq!(
            crate::upk::chunks::decompress_all(&pkg.bytes, &map).unwrap(),
            payload
        );
        let mut out = pkg.header_bytes().unwrap();
        pkg.seal(&mut out).unwrap();
        assert_eq!(out, file);
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
