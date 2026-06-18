use storage_engine::{Engine, EngineConfig, Result};

fn main() -> Result<()> {
    let mut config = EngineConfig::new(std::env::temp_dir().join("storage-engine-runner"));
    config.memtable_threshold = 16;

    let mut engine = Engine::new(config);

    engine.put(b"alpha".to_vec(), b"one".to_vec())?;
    engine.put(b"beta".to_vec(), b"two".to_vec())?;
    engine.delete(b"alpha".to_vec())?;

    println!("alpha = {:?}", engine.get(b"alpha")?);
    println!("beta = {:?}", engine.get(b"beta")?);
    println!("sstables = {}", engine.sstable_count());
    println!("wal_records = {}", engine.wal_records().len());

    Ok(())
}
