# Package encryption (ECB and CTR)

Rocket League cooked packages come in two encrypted flavours. Both use the
AES-256 keys of `keys.txt`; the game picks the key from the package **name**,
so a package written under another name must be re-encrypted with that
name's key (`upk::rename::rename_package(.., target_key)`,
`decals::pipeline::reencrypt_for_target`).

## ECB (most packages)

- Region: `[NameOffset, NameOffset + ((TotalHeaderSize - NameOffset) & !15))`,
  i.e. the name / import / export / depends tables and the chunk table.
- AES-256-ECB, block by block. The compressed body is plain.

## CTR — "fully encrypted" packages (since the 2026-08 patch)

Detected by `LicenseeVersion >= 33` **and** package flag `0x0800`
(`summary::FULL_ENCRYPTED_FLAG`). Seen on new items (season 24 bodies,
wheels, boosts…); 205 of 18,142 packages on the 2026-09-23 build.

- Summary tail (licensee 33+): after `GarbageSize/TestDataSize`,
  `CompressedChunkInfoOffset`, `LastBlockSize` come three `u32` = the
  12-byte **header nonce**. The encrypted region starts right after, at
  `NameOffset`, so the nonce is `file[NameOffset-12 .. NameOffset]`.
- Header: AES-256-CTR, counter block = `nonce(12) ‖ u32 BE counter`,
  counter 0 at `NameOffset`. Same region length as ECB.
- Chunk table (in the decrypted header, at `NameOffset +
  CompressedChunkInfoOffset`): `i32 count`, then 36-byte entries
  `i64 u_off · i32 u_size · i64 c_off · i32 c_size · u8[12] nonce`.
  In ECB packages the 12 trailing bytes exist too (unused).
- Every compressed chunk `[c_off, c_off + c_size)` is AES-256-CTR with its
  own nonce, counter 0 at `c_off`. Decrypted, it is a normal chunk
  (`0x9E2A83C1`, block size, sizes, zlib blocks).
- `upk::Package::open` decrypts the chunks in memory; `Package::seal`
  re-encrypts them (with the possibly re-targeted key) after body patches.

Reference: ShinyEmii's UEViewer fork (`UnPackageReader.cpp`, commits
2026-08-27 "Licensee version 33 and automatic key detection" and
2026-09-23 "Patch for LicenseeVer 34"). Key list: ShinyEmii/Toga-Files
`aes.txt`, updated each patch; packages whose key is not public yet are
listed in its `missed_packages.txt` (usually unreleased items).
