pub mod record;

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::format::{Decoder, Serializable};
use crate::storage::manifest::record::ManifestRecord;
use crate::storage::sstable::meta::SSTableMeta;
use crate::Result;

const MANIFEST_DIR: &str = "manifest";
const MANIFEST_FILE_EXTENSION: &str = "manifest";

#[derive(Debug)]
pub struct ManifestManager {
    data_dir: PathBuf,
    manifest_dir: PathBuf,
    current_manifest_id: u64,
    next_manifest_id: u64,
    current_file: File,
}

impl ManifestManager {
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        let manifest_dir = data_dir.join(MANIFEST_DIR);
        std::fs::create_dir_all(&manifest_dir)?;

        let current_manifest_id = next_manifest_id(&manifest_dir)?;
        Self::open_with_next_manifest_id(data_dir, current_manifest_id)
    }

    pub fn open_with_next_manifest_id(
        data_dir: impl Into<PathBuf>,
        next_manifest_id: u64,
    ) -> Result<Self> {
        let data_dir = data_dir.into();
        let manifest_dir = data_dir.join(MANIFEST_DIR);
        std::fs::create_dir_all(&manifest_dir)?;
        let current_manifest_id = next_manifest_id;
        let current_file = open_manifest_file(&manifest_path(&manifest_dir, current_manifest_id))?;

        Ok(Self {
            data_dir,
            manifest_dir,
            current_manifest_id,
            next_manifest_id: current_manifest_id + 1,
            current_file,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn current_manifest_id(&self) -> u64 {
        self.current_manifest_id
    }

    pub fn next_manifest_id(&self) -> u64 {
        self.next_manifest_id
    }

    pub fn append(&mut self, record: &ManifestRecord) -> Result<()> {
        self.current_file.write_all(&record.encode())?;
        self.current_file.sync_data()?;
        Ok(())
    }

    pub fn add_sstable(&mut self, meta: SSTableMeta) -> Result<()> {
        self.append(&ManifestRecord::AddSSTable(meta))
    }

    pub fn remove_sstable(&mut self, file_id: u64) -> Result<()> {
        self.append(&ManifestRecord::RemoveSSTable { file_id })
    }

    pub fn rotate(&mut self) -> Result<()> {
        self.current_manifest_id = self.next_manifest_id;
        self.next_manifest_id += 1;
        self.current_file =
            open_manifest_file(&manifest_path(&self.manifest_dir, self.current_manifest_id))?;
        Ok(())
    }

    pub fn replay(&self) -> Result<Vec<ManifestRecord>> {
        replay_manifest_dir(&self.manifest_dir)
    }

    pub fn load_sstables(&self) -> Result<Vec<SSTableMeta>> {
        Ok(apply_records(self.replay()?).into_values().collect())
    }

    pub fn sstable_path(&self, file_id: u64) -> PathBuf {
        self.data_dir.join(format!("{file_id:020}.sst"))
    }

    pub fn next_sstable_id(&self) -> Result<u64> {
        Ok(self
            .load_sstables()?
            .into_iter()
            .map(|meta| meta.file_id())
            .max()
            .map_or(0, |file_id| file_id + 1))
    }

    pub fn compact(&mut self) -> Result<()> {
        let sstables = self.load_sstables()?;
        self.current_manifest_id = self.next_manifest_id;
        self.next_manifest_id += 1;
        let compacted_path = manifest_path(&self.manifest_dir, self.current_manifest_id);
        let mut compacted_file = open_manifest_file(&compacted_path)?;

        for meta in &sstables {
            compacted_file.write_all(&ManifestRecord::AddSSTable(meta.clone()).encode())?;
        }
        compacted_file.sync_data()?;

        self.current_file = compacted_file;
        remove_older_manifests(&self.manifest_dir, self.current_manifest_id)?;
        Ok(())
    }
}

pub fn replay_manifest_dir(manifest_dir: impl AsRef<Path>) -> Result<Vec<ManifestRecord>> {
    let mut records = Vec::new();

    for path in manifest_paths(manifest_dir.as_ref())? {
        let bytes = std::fs::read(path)?;
        let mut decoder = Decoder::new(&bytes);

        while !decoder.is_finished() {
            records.push(decoder.read_serializable()?);
        }
    }

    Ok(records)
}

fn apply_records(records: impl IntoIterator<Item = ManifestRecord>) -> BTreeMap<u64, SSTableMeta> {
    let mut sstables = BTreeMap::new();

    for record in records {
        match record {
            ManifestRecord::AddSSTable(meta) => {
                sstables.insert(meta.file_id(), meta);
            }
            ManifestRecord::RemoveSSTable { file_id } => {
                sstables.remove(&file_id);
            }
        }
    }

    sstables
}

fn next_manifest_id(manifest_dir: &Path) -> Result<u64> {
    Ok(manifest_paths(manifest_dir)?
        .into_iter()
        .filter_map(|path| manifest_id(&path))
        .max()
        .map_or(0, |id| id + 1))
}

fn manifest_paths(manifest_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in std::fs::read_dir(manifest_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some(MANIFEST_FILE_EXTENSION) {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn manifest_id(path: &Path) -> Option<u64> {
    path.file_stem()?.to_str()?.parse().ok()
}

fn manifest_path(manifest_dir: &Path, manifest_id: u64) -> PathBuf {
    manifest_dir.join(format!("{manifest_id:020}.{MANIFEST_FILE_EXTENSION}"))
}

fn open_manifest_file(path: &Path) -> Result<File> {
    Ok(OpenOptions::new().create(true).append(true).open(path)?)
}

fn remove_older_manifests(manifest_dir: &Path, current_manifest_id: u64) -> Result<()> {
    for path in manifest_paths(manifest_dir)? {
        if manifest_id(&path).is_some_and(|id| id < current_manifest_id) {
            std::fs::remove_file(path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}"))
    }

    fn meta(file_id: u64) -> SSTableMeta {
        SSTableMeta::new(
            file_id,
            0,
            format!("key-{file_id}-a").into_bytes(),
            format!("key-{file_id}-z").into_bytes(),
            4096,
        )
    }

    #[test]
    fn appends_and_replays_records() {
        let dir = temp_dir("manifest-replay");
        let mut manager = ManifestManager::open(&dir).unwrap();

        manager.add_sstable(meta(1)).unwrap();
        manager.remove_sstable(1).unwrap();

        assert_eq!(
            manager.replay().unwrap(),
            vec![
                ManifestRecord::AddSSTable(meta(1)),
                ManifestRecord::RemoveSSTable { file_id: 1 }
            ]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_sstables_applies_adds_and_removes() {
        let dir = temp_dir("manifest-load");
        let mut manager = ManifestManager::open(&dir).unwrap();

        manager.add_sstable(meta(1)).unwrap();
        manager.add_sstable(meta(2)).unwrap();
        manager.remove_sstable(1).unwrap();

        assert_eq!(manager.load_sstables().unwrap(), vec![meta(2)]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rotate_writes_to_new_manifest_file() {
        let dir = temp_dir("manifest-rotate");
        let mut manager = ManifestManager::open(&dir).unwrap();

        assert_eq!(manager.current_manifest_id(), 0);
        assert_eq!(manager.next_manifest_id(), 1);
        manager.add_sstable(meta(1)).unwrap();
        manager.rotate().unwrap();
        assert_eq!(manager.current_manifest_id(), 1);
        assert_eq!(manager.next_manifest_id(), 2);
        manager.add_sstable(meta(2)).unwrap();

        assert_eq!(
            manager.replay().unwrap(),
            vec![
                ManifestRecord::AddSSTable(meta(1)),
                ManifestRecord::AddSSTable(meta(2))
            ]
        );
        assert_eq!(manifest_paths(&dir.join(MANIFEST_DIR)).unwrap().len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn compact_keeps_live_sstables_and_removes_old_files() {
        let dir = temp_dir("manifest-compact");
        let mut manager = ManifestManager::open(&dir).unwrap();

        manager.add_sstable(meta(1)).unwrap();
        manager.rotate().unwrap();
        manager.add_sstable(meta(2)).unwrap();
        manager.remove_sstable(1).unwrap();
        manager.compact().unwrap();

        assert_eq!(manager.load_sstables().unwrap(), vec![meta(2)]);
        assert_eq!(manifest_paths(&dir.join(MANIFEST_DIR)).unwrap().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn compacted_manifest_remains_appendable() {
        let dir = temp_dir("manifest-compact-appendable");
        let mut manager = ManifestManager::open(&dir).unwrap();

        manager.add_sstable(meta(1)).unwrap();
        manager.compact().unwrap();
        manager.add_sstable(meta(2)).unwrap();

        assert_eq!(manager.load_sstables().unwrap(), vec![meta(1), meta(2)]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn next_sstable_id_advances_from_manifest_state() {
        let dir = temp_dir("manifest-next-sstable-id");
        let mut manager = ManifestManager::open(&dir).unwrap();

        assert_eq!(manager.next_sstable_id().unwrap(), 0);
        manager.add_sstable(meta(7)).unwrap();
        manager.add_sstable(meta(3)).unwrap();

        assert_eq!(manager.next_sstable_id().unwrap(), 8);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn next_manifest_id_advances_after_compaction() {
        let dir = temp_dir("manifest-next-manifest-id");
        let mut manager = ManifestManager::open(&dir).unwrap();

        manager.add_sstable(meta(1)).unwrap();
        manager.compact().unwrap();

        assert_eq!(manager.current_manifest_id(), 1);
        assert_eq!(manager.next_manifest_id(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
