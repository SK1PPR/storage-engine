use crate::{
    current::{current_path, CurrentFile},
    index::{
        memtable_manager::{MemTableManager, MemTableWriteOutcome},
        Key,
    },
    storage::{
        manifest::ManifestManager,
        sstable::{manager::SSTableManager, meta::SSTableMeta},
    },
    wal::{WalManager, WalRecord},
    EngineConfig, Result,
};
use std::path::Path;

pub struct Engine {
    current: CurrentFile,
    wal: WalManager,
    memtables: MemTableManager,
    manifest_manager: ManifestManager,
    sstables: SSTableManager,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        let wal_dir = config.data_dir.join("wal");
        let current_exists = current_path(&config.data_dir).exists();
        let mut current = CurrentFile::open(&config.data_dir).expect("failed to open CURRENT file");
        let manifest_manager = if current_exists {
            ManifestManager::open_with_next_manifest_id(
                &config.data_dir,
                current.state().next_manifest_id,
            )
            .expect("failed to open manifest manager")
        } else {
            ManifestManager::open(&config.data_dir).expect("failed to open manifest manager")
        };
        let manifest_next_sstable_id = manifest_manager
            .next_sstable_id()
            .expect("failed to load manifest state");
        let next_sstable_id = current
            .state()
            .next_sstable_id
            .max(manifest_next_sstable_id);
        let sstables = SSTableManager::new(&config.data_dir, next_sstable_id);
        let recovered_active_wal_id = current.state().next_wal_id.saturating_sub(1);
        let mut wal = WalManager::open(
            wal_dir,
            current.state().next_wal_id,
            current.state().next_sequence_id,
        )
        .expect("failed to create WAL directory");
        let mut memtables =
            MemTableManager::new(config.memtable_threshold, config.maximum_memtables);
        let mut next_sequence_id = wal.next_sequence_id();

        for segment in wal.replay_segments().expect("failed to replay WAL records") {
            for record in &segment.records {
                next_sequence_id = next_sequence_id.max(record.sequence_id() + 1);
            }

            if segment.wal_id < recovered_active_wal_id {
                memtables.recover_immutable_segment(segment.records);
            } else {
                memtables.recover_active_segment(segment.records);
            }
        }
        wal.set_next_sequence_id(next_sequence_id);

        current.state_mut().next_sstable_id = next_sstable_id;
        current.state_mut().next_wal_id = wal.next_wal_id();
        current.state_mut().next_manifest_id = manifest_manager.next_manifest_id();
        current.state_mut().next_sequence_id = next_sequence_id;
        current.persist().expect("failed to persist CURRENT file");

        Self {
            current,
            wal,
            memtables,
            manifest_manager,
            sstables,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let sequence_id = self.wal.next_sequence();
        self.wal
            .append(WalRecord::put(sequence_id, key.clone(), value.clone()))?;
        self.current.state_mut().next_sequence_id = self.wal.next_sequence_id();
        self.current.persist()?;
        let outcome = self.memtables.put(key, value);
        self.handle_memtable_write(outcome)?;

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let key = Key::new(key);

        if let Some(value) = self.memtables.get(&key) {
            return Ok(value.as_bytes().map(ToOwned::to_owned));
        }

        let sstables = self.manifest_manager.load_sstables()?;
        Ok(self
            .sstables
            .get(&key, &sstables)?
            .and_then(|value| value.as_bytes().map(ToOwned::to_owned)))
    }

    pub fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        let sequence_id = self.wal.next_sequence();
        self.wal
            .append(WalRecord::delete(sequence_id, key.clone()))?;
        self.current.state_mut().next_sequence_id = self.wal.next_sequence_id();
        self.current.persist()?;
        let outcome = self.memtables.delete(key);
        self.handle_memtable_write(outcome)?;

        Ok(())
    }

    fn handle_memtable_write(&mut self, outcome: MemTableWriteOutcome) -> Result<()> {
        if outcome.rotated {
            self.wal.rotate();
            self.current.state_mut().next_wal_id = self.wal.next_wal_id();
            self.current.persist()?;
        }

        if outcome.should_flush {
            self.flush_memtable()?;
        }
        Ok(())
    }

    pub fn flush_memtable(&mut self) -> Result<()> {
        let Some(memtable) = self.memtables.pop_flushable() else {
            return Ok(());
        };

        if let Some(meta) = self.sstables.write_memtable(&memtable)? {
            self.manifest_manager.add_sstable(meta)?;
            self.current.state_mut().next_sstable_id = self.sstables.next_sstable_id();
            self.current.persist()?;
        }

        if let Some(wal) = self.wal.pop_oldest() {
            wal.remove_file()?;
        }

        Ok(())
    }

    pub fn sstable_count(&self) -> Result<usize> {
        Ok(self.manifest_manager.load_sstables()?.len())
    }

    pub fn memtable_size(&self) -> usize {
        self.memtables.active_size()
    }

    pub fn immutable_memtable_count(&self) -> usize {
        self.memtables.immutable_count()
    }

    pub fn data_dir(&self) -> &Path {
        self.sstables.data_dir()
    }

    pub fn manifest_sstables(&self) -> Result<Vec<SSTableMeta>> {
        self.manifest_manager.load_sstables()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> EngineConfig {
        isolated_config("storage-engine-tests")
    }

    fn config_with_immutable_queue() -> EngineConfig {
        let mut config = isolated_config("storage-engine-immutable-tests");
        config.memtable_threshold = 8;
        config.maximum_memtables = 4;
        config
    }

    fn isolated_config(name: &str) -> EngineConfig {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut config = EngineConfig::new(std::env::temp_dir().join(format!("{name}-{nanos}")));
        config.memtable_threshold = 8;
        config.maximum_memtables = 0;
        config
    }

    #[test]
    fn put_and_get_from_active_memtable() {
        let mut engine = Engine::new(config());
        engine.put(b"a".to_vec(), b"1".to_vec()).unwrap();

        assert_eq!(engine.get(b"a").unwrap(), Some(b"1".to_vec()));
    }

    #[test]
    fn flushes_filled_memtable_to_sstable_and_reads_from_it() {
        let mut engine = Engine::new(config());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();

        assert_eq!(engine.sstable_count().unwrap(), 1);
        assert_eq!(engine.memtable_size(), 0);
        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"one".to_vec()));
    }

    #[test]
    fn newest_value_wins_across_sstables() {
        let mut engine = Engine::new(config());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        engine.put(b"alpha".to_vec(), b"two".to_vec()).unwrap();

        assert_eq!(engine.sstable_count().unwrap(), 2);
        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"two".to_vec()));
    }

    #[test]
    fn delete_hides_older_values() {
        let mut engine = Engine::new(config());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        engine.delete(b"alpha".to_vec()).unwrap();

        assert_eq!(engine.get(b"alpha").unwrap(), None);
    }

    #[test]
    fn reads_from_immutable_memtable_before_flush() {
        let mut engine = Engine::new(config_with_immutable_queue());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();

        assert_eq!(engine.sstable_count().unwrap(), 0);
        assert_eq!(engine.immutable_memtable_count(), 1);
        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"one".to_vec()));
    }

    #[test]
    fn flush_records_sstable_in_manifest() {
        let config = isolated_config("storage-engine-manifest");
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config);

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();

        let sstables = engine.manifest_sstables().unwrap();
        assert_eq!(sstables.len(), 1);
        assert_eq!(sstables[0].file_id(), 0);
        assert_eq!(sstables[0].smallest_key(), b"alpha");
        assert_eq!(sstables[0].largest_key(), b"alpha");

        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn reopened_engine_reads_sstables_from_manifest() {
        let config = isolated_config("storage-engine-reopen-manifest");
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        drop(engine);

        let mut reopened = Engine::new(config);

        assert_eq!(reopened.get(b"alpha").unwrap(), Some(b"one".to_vec()));
        reopened.put(b"beta".to_vec(), b"two".to_vec()).unwrap();
        let sstables = reopened.manifest_sstables().unwrap();
        assert_eq!(sstables.len(), 2);
        assert_eq!(sstables[0].file_id(), 0);
        assert_eq!(sstables[1].file_id(), 1);

        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn current_file_tracks_recovery_ids() {
        let config = isolated_config("storage-engine-current");
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config);

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();

        let current = CurrentFile::open(&data_dir).unwrap();
        assert_eq!(current.state().next_sstable_id, 1);
        assert_eq!(current.state().next_wal_id, 2);
        assert_eq!(current.state().next_manifest_id, 1);
        assert_eq!(current.state().next_sequence_id, 2);

        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn reopened_engine_uses_current_file_for_next_ids() {
        let config = isolated_config("storage-engine-current-reopen");
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        drop(engine);

        let mut reopened = Engine::new(config);
        reopened.put(b"beta".to_vec(), b"two".to_vec()).unwrap();

        let current = CurrentFile::open(&data_dir).unwrap();
        let sstables = reopened.manifest_sstables().unwrap();
        assert_eq!(
            sstables
                .iter()
                .map(|meta| meta.file_id())
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(current.state().next_sstable_id, 2);
        assert_eq!(current.state().next_wal_id, 4);
        assert!(current_path(&data_dir).exists());

        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn recovers_unflushed_put_from_wal() {
        let mut config = isolated_config("storage-engine-recover-put");
        config.memtable_threshold = 4096;
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        drop(engine);

        let recovered = Engine::new(config);

        assert_eq!(recovered.get(b"alpha").unwrap(), Some(b"one".to_vec()));
        assert_eq!(recovered.sstable_count().unwrap(), 0);
        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn recovers_pre_crash_active_wal_as_active_memtable() {
        let mut config = isolated_config("storage-engine-recover-active-wal");
        config.memtable_threshold = 4096;
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        drop(engine);

        let recovered = Engine::new(config);

        assert_eq!(recovered.get(b"alpha").unwrap(), Some(b"one".to_vec()));
        assert!(recovered.memtable_size() > 0);
        assert_eq!(recovered.immutable_memtable_count(), 0);
        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn recovers_rotated_wal_as_immutable_memtable() {
        let mut config = isolated_config("storage-engine-recover-immutable-wal");
        config.memtable_threshold = 8;
        config.maximum_memtables = 4;
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        drop(engine);

        let recovered = Engine::new(config);

        assert_eq!(recovered.get(b"alpha").unwrap(), Some(b"one".to_vec()));
        assert_eq!(recovered.memtable_size(), 0);
        assert_eq!(recovered.immutable_memtable_count(), 1);
        assert_eq!(recovered.sstable_count().unwrap(), 0);
        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn recovers_unflushed_delete_from_wal() {
        let config = isolated_config("storage-engine-recover-delete");
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        engine.delete(b"alpha".to_vec()).unwrap();
        drop(engine);

        let recovered = Engine::new(config);

        assert_eq!(recovered.get(b"alpha").unwrap(), None);
        assert_eq!(recovered.sstable_count().unwrap(), 1);
        std::fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn recovered_wal_advances_next_sequence_id() {
        let mut config = isolated_config("storage-engine-recover-sequence");
        config.memtable_threshold = 4096;
        let data_dir = config.data_dir.clone();
        let mut engine = Engine::new(config.clone());

        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        drop(engine);

        let mut recovered = Engine::new(config);
        recovered.put(b"beta".to_vec(), b"two".to_vec()).unwrap();

        let current = CurrentFile::open(&data_dir).unwrap();
        assert_eq!(current.state().next_sequence_id, 3);
        std::fs::remove_dir_all(data_dir).unwrap();
    }
}
