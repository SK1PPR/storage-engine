use storage_engine::{Engine, EngineConfig, Result};

fn main() -> Result<()> {
    let data_dir = std::env::temp_dir().join("storage-engine-runner");
    let mut config = EngineConfig::new(&data_dir);
    config.memtable_threshold = 24;

    let mut engine = Engine::new(config);

    println!("storage engine demo");
    println!("data_dir = {}", engine.data_dir().display());

    engine.put(b"alpha".to_vec(), b"one".to_vec())?;
    engine.put(b"beta".to_vec(), b"two".to_vec())?;
    engine.put(b"gamma".to_vec(), b"three".to_vec())?;
    engine.put(b"alpha".to_vec(), b"updated".to_vec())?;
    engine.delete(b"alpha".to_vec())?;
    engine.flush_memtable()?;

    print_get(&engine, "alpha")?;
    print_get(&engine, "beta")?;
    print_get(&engine, "gamma")?;
    print_get(&engine, "missing")?;

    println!("sstables = {}", engine.sstable_count());
    println!("wal_records = {}", engine.wal_records().len());
    println!("files:");

    for entry in std::fs::read_dir(&data_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        println!("  {} ({} bytes)", entry.path().display(), metadata.len());
    }

    Ok(())
}

fn print_get(engine: &Engine, key: &str) -> Result<()> {
    let value = engine
        .get(key.as_bytes())?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    println!("{key} = {value:?}");
    Ok(())
}
