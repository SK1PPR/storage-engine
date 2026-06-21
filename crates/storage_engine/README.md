# pocket-lsm

`pocket-lsm` is a small educational LSM storage engine written in Rust.

It currently includes:

- skip-list memtables
- write-ahead log segments
- SSTable files with block indexes and Bloom filters
- manifest records and manifest compaction
- a `CURRENT` recovery metadata file
- crash recovery from WAL segments

This crate is experimental and not production-ready. It exists as a compact place to explore storage-engine internals before deeper optimization work.
