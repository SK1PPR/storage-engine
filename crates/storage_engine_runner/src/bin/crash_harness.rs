use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use pocket_lsm::{Engine, EngineConfig, Result};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("write-loop") => write_loop(&args),
        Some("verify") => verify(&args),
        _ => {
            eprintln!(
                "usage: crash_harness write-loop <data-dir> <memtable-threshold> | verify <data-dir> <expected-file>"
            );
            std::process::exit(2);
        }
    }
}

fn write_loop(args: &[String]) -> Result<()> {
    let data_dir = arg_path(args, 2);
    let memtable_threshold = arg_usize(args, 3);
    let mut engine = Engine::new(config(&data_dir, memtable_threshold));
    let mut stdout = std::io::stdout();
    let mut index = 0_u64;

    loop {
        let key = format!("key-{index:020}");
        let value = format!("value-{index:020}");
        engine.put(key.as_bytes().to_vec(), value.as_bytes().to_vec())?;
        writeln!(stdout, "PUT\t{key}\t{value}")?;
        stdout.flush()?;
        index += 1;
        thread::sleep(Duration::from_millis(5));
    }
}

fn verify(args: &[String]) -> Result<()> {
    let data_dir = arg_path(args, 2);
    let expected_file = arg_path(args, 3);
    let engine = Engine::new(config(&data_dir, 64));
    let expected = std::fs::read_to_string(expected_file)?;

    for line in expected.lines() {
        let mut parts = line.split('\t');
        let key = parts.next().expect("expected key column");
        let value = parts.next().expect("expected value column");
        let actual = engine.get(key.as_bytes())?;
        assert_eq!(
            actual.as_deref(),
            Some(value.as_bytes()),
            "recovered value mismatch for {key}"
        );
    }

    println!("verified {} keys", expected.lines().count());
    Ok(())
}

fn config(data_dir: &Path, memtable_threshold: usize) -> EngineConfig {
    let mut config = EngineConfig::new(data_dir);
    config.memtable_threshold = memtable_threshold;
    config.maximum_memtables = 4;
    config
}

fn arg_path(args: &[String], index: usize) -> PathBuf {
    args.get(index)
        .unwrap_or_else(|| panic!("missing argument {index}"))
        .into()
}

fn arg_usize(args: &[String], index: usize) -> usize {
    args.get(index)
        .unwrap_or_else(|| panic!("missing argument {index}"))
        .parse()
        .unwrap_or_else(|_| panic!("argument {index} must be a usize"))
}
