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
- [x] Add runner crate for basic manual execution.

## 2. File-Backed SSTables

- [x] Define SSTable binary file layout.
- [x] Implement data block encoding.
- [x] Implement index block encoding.
- [x] Implement footer encoding with magic number.
- [x] Implement `SsTableWriter`.
- [x] Implement `SsTableReader`.
- [x] Replace in-memory `SsTable` entries with file-backed lookup.
- [x] Add tests for write, reopen, get, miss, tombstone, and sorted iteration.

## 3. WAL Durability

- [x] Replace in-memory WAL with file-backed WAL.
- [x] Encode `Put` and `Delete` records.
- [ ] Add record length and checksum.
- [ ] Replay WAL on engine startup.
- [ ] Truncate or rotate WAL after successful MemTable flush.
- [ ] Add crash-recovery tests.

## 4. Immutable MemTables

- [ ] Add active MemTable plus immutable MemTable list.
- [ ] Rotate active MemTable when threshold is crossed.
- [ ] Keep immutable MemTables readable until flush completes.
- [ ] Flush immutable MemTables to SSTables.
- [ ] Remove immutable MemTables after successful SSTable install.

## 5. Manifest And Metadata

- [ ] Add manifest file.
- [ ] Track SSTable ids, paths, levels, and key ranges.
- [ ] Persist atomic SSTable creation.
- [ ] Recover SSTable set from manifest on startup.
- [ ] Add tests for restart with existing SSTables.

## 6. Compaction

- [ ] Add level-0 SSTable organization.
- [ ] Implement sorted merge iterator.
- [ ] Drop overwritten values and obsolete tombstones.
- [ ] Write compacted SSTables.
- [ ] Atomically swap old SSTables for compacted SSTables.
- [ ] Add basic compaction trigger.

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

