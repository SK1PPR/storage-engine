# Storage Engine Checklist

## 1. Working Baseline

- [x] Create Rust workspace and storage engine library crate.
- [x] Add standard `Key` and `Value` types.
- [x] Add `MemTable` trait.
- [x] Implement `BTree` MemTable.
- [x] Implement `SkipList` MemTable.
- [x] Add basic in-memory WAL records.
- [x] Add synchronous `Engine`.
- [x] Support `put`, `get`, and `delete`.
- [x] Flush filled active MemTable into an immutable SSTable vector.
- [x] Refactor `Engine` to orchestrate WAL, MemTable, SSTable, and Manifest managers.
- [x] Add runner crate for basic manual execution.

## 2. File-Backed SSTables

- [x] Define SSTable binary file layout.
- [x] Implement data block encoding.
- [x] Implement index block encoding.
- [x] Implement footer encoding with magic number.
- [x] Implement `SsTableWriter`.
- [x] Implement `SsTableReader`.
- [x] Add `SSTableManager` for SSTable ids, paths, writes, and reads.
- [x] Replace in-memory `SsTable` entries with file-backed lookup.
- [x] Add tests for write, reopen, get, miss, tombstone, and sorted iteration.

## 3. WAL Durability

- [x] Add `CURRENT` file for recovered SSTable, WAL, manifest, and sequence ids.
- [x] Replace in-memory WAL with file-backed WAL.
- [x] Encode `Put` and `Delete` records.
- [x] Add record length and checksum.
- [x] Remove in-memory WAL record cache; use disk WAL as source of truth.
- [ ] Replay WAL on engine startup.
- [x] Truncate or rotate WAL after successful MemTable flush.
- [ ] Add crash-recovery tests.
- [x] Recover WAL manager sequence state from `CURRENT` after restart.
- [ ] Reconcile flushed SSTables and unflushed WAL segments after crash.

## 4. Immutable MemTables

- [x] Add active MemTable plus immutable MemTable list.
- [x] Rotate active MemTable when threshold is crossed.
- [x] Add `MemTableManager` for active MemTable, immutable MemTables, and flush policy.
- [x] Keep immutable MemTables readable until flush completes.
- [x] Flush immutable MemTables to SSTables.
- [x] Remove immutable MemTables after successful SSTable install.

## 5. Manifest And Metadata

- [x] Add manifest record encoding and decoding.
- [x] Add `ManifestManager` with per-data-dir manifest segment files.
- [x] Track SSTable ids, levels, file sizes, and key ranges.
- [x] Persist SSTable creation by appending manifest records after flush.
- [x] Recover SSTable set from manifest on startup.
- [x] Use manifest metadata as the engine's SSTable source of truth.
- [x] Add tests for restart with existing SSTables.
- [x] Add manifest segment rotation tests.
- [x] Add manifest compaction tests.
- [ ] Make SSTable creation fully atomic across temp-file write, fsync, rename, and manifest append.

## 6. Compaction

- [ ] Add level-0 SSTable organization.
- [ ] Implement sorted merge iterator.
- [ ] Drop overwritten values and obsolete tombstones.
- [ ] Write compacted SSTables.
- [ ] Atomically swap old SSTables for compacted SSTables.
- [ ] Add basic compaction trigger.
- [ ] Hook compaction output into `ManifestManager` add/remove records.
- [ ] Add tests for newest-value wins after compaction.
- [ ] Add tests for tombstone handling after compaction.

## 7. Async Workflow

- [ ] Introduce async runtime only after sync path is stable.
- [ ] Add background WAL writer.
- [ ] Add background MemTable flush worker.
- [ ] Add background compaction worker.
- [ ] Add clean shutdown.
- [ ] Add durability modes: async, flush, fsync.

## 8. Block Cache

- [ ] Add block cache keyed by SSTable id and block offset.
- [ ] Implement LRU or CLOCK eviction.
- [ ] Track cache hits and misses.
- [ ] Add cache admission policy.
- [ ] Benchmark read-heavy and mixed workloads.

## 9. Bloom Filters

- [x] Add Bloom filter per SSTable.
- [x] Serialize Bloom filter into SSTable file.
- [x] Use Bloom filter to skip definitely-missing keys.
- [ ] Benchmark miss-heavy workloads.

## 10. Adaptive Radix Tree

- [ ] Add ART module.
- [ ] Implement Node4.
- [ ] Implement Node16.
- [ ] Implement Node48.
- [ ] Implement Node256.
- [ ] Add path compression.
- [ ] Implement `MemTable` for ART.
- [ ] Benchmark ART against SkipList and BTree.

## 11. eBPF Integration

- [ ] Define hot block and hot key-range metrics.
- [ ] Add user-space cache telemetry first.
- [ ] Design pinned eBPF map schema.
- [ ] Publish hot SSTable block metadata into eBPF maps.
- [ ] Read eBPF hints in cache admission or prefetch logic.
- [ ] Benchmark with and without eBPF-guided caching.

## 12. Benchmarking

- [ ] Add internal benchmark harness.
- [ ] Compare BTree MemTable vs SkipList MemTable.
- [ ] Compare against RocksDB.
- [ ] Compare against fjall.
- [ ] Test write-heavy workload.
- [ ] Test read-heavy workload.
- [ ] Test mixed workload.
- [ ] Test range scan workload.
- [ ] Test Zipfian hot-key workload.
- [ ] Test cold random-read workload.

## 13. Advanced Performance Roadmap

- [ ] Add `IoBackend` abstraction around offset reads, offset writes, and fsync.
- [ ] Implement blocking `pread`/`pwrite` backend before introducing `io_uring`.
- [ ] Add `io_uring` backend with registered files and fixed buffers.
- [ ] Pipeline compaction reads, merges, writes, and fsync through async jobs.
- [ ] Add WAL group commit with configurable durability policy.
- [ ] Chain batched WAL write and fsync operations when using `io_uring`.
- [ ] Replace SkipList MemTable with arena-backed ART.
- [ ] Use arenas for MemTable key/value/node allocation.
- [ ] Add streaming SSTable block iterators to avoid materializing full entry vectors.
- [ ] Add prefix-compressed SSTable data blocks with restart points.
- [ ] Add partitioned SSTable indexes and filters.
- [ ] Evaluate blocked Bloom filters for cache-line-local point lookups.
- [ ] Evaluate Ribbon or Xor filters for static SSTable membership filters.
- [ ] Evaluate range filters for range-scan-heavy workloads.
- [ ] Add separate caches for data blocks, index/filter metadata, table readers, and misses.
- [ ] Evaluate TinyLFU or W-TinyLFU cache admission instead of plain LRU.
- [ ] Add leveled, tiered/universal, and hybrid compaction policies.
- [ ] Add compaction debt accounting and write-stall control.
- [ ] Add parallel subcompactions over disjoint key ranges.
- [ ] Evaluate key-value separation for large values.
- [ ] Use eBPF for boundary-level features: request steering, shared pinned hints, I/O admission, or io_uring diagnostics.
