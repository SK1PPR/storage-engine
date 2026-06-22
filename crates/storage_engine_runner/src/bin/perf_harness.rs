use std::path::{Path, PathBuf};
use std::time::Instant;

use pocket_lsm::{Engine, EngineConfig, Result};

#[derive(Debug)]
struct Options {
    workload: Workload,
    data_dir: PathBuf,
    keys: u64,
    ops: u64,
    value_bytes: usize,
    memtable_threshold: usize,
    maximum_memtables: usize,
    read_percent: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Workload {
    Write,
    Populate,
    Read,
    Mixed,
}

fn main() -> Result<()> {
    let options = Options::parse();
    let start = Instant::now();
    let mut engine = Engine::new(config(
        &options.data_dir,
        options.memtable_threshold,
        options.maximum_memtables,
    ));

    let completed_ops = match options.workload {
        Workload::Write | Workload::Populate => run_write(&mut engine, &options)?,
        Workload::Read => run_read(&engine, &options)?,
        Workload::Mixed => run_mixed(&mut engine, &options)?,
    };

    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    let ops_per_sec = if elapsed_secs == 0.0 {
        0.0
    } else {
        completed_ops as f64 / elapsed_secs
    };

    println!(
        "{{\"workload\":\"{}\",\"ops\":{},\"elapsed_ms\":{},\"ops_per_sec\":{:.2},\"keys\":{},\"value_bytes\":{},\"memtable_threshold\":{},\"maximum_memtables\":{},\"memtable_size\":{},\"immutable_memtables\":{},\"sstables\":{}}}",
        options.workload.as_str(),
        completed_ops,
        elapsed.as_millis(),
        ops_per_sec,
        options.keys,
        options.value_bytes,
        options.memtable_threshold,
        options.maximum_memtables,
        engine.memtable_size(),
        engine.immutable_memtable_count(),
        engine.sstable_count()?,
    );

    Ok(())
}

fn run_write(engine: &mut Engine, options: &Options) -> Result<u64> {
    for index in 0..options.keys {
        engine.put(make_key(index), make_value(index, options.value_bytes))?;
    }

    Ok(options.keys)
}

fn run_read(engine: &Engine, options: &Options) -> Result<u64> {
    let ops = options.ops.max(options.keys);

    for op in 0..ops {
        let key_index = pseudo_random_index(op, options.keys);
        let value = engine.get(&make_key(key_index))?;
        if value.is_none() {
            panic!("missing value for key index {key_index}");
        }
    }

    Ok(ops)
}

fn run_mixed(engine: &mut Engine, options: &Options) -> Result<u64> {
    for op in 0..options.ops {
        if (op % 100) < options.read_percent as u64 && options.keys > 0 {
            let key_index = pseudo_random_index(op, options.keys);
            let _ = engine.get(&make_key(key_index))?;
        } else {
            let key_index = options.keys + op;
            engine.put(
                make_key(key_index),
                make_value(key_index, options.value_bytes),
            )?;
        }
    }

    Ok(options.ops)
}

fn config(data_dir: &Path, memtable_threshold: usize, maximum_memtables: usize) -> EngineConfig {
    let mut config = EngineConfig::new(data_dir);
    config.memtable_threshold = memtable_threshold;
    config.maximum_memtables = maximum_memtables;
    config
}

fn make_key(index: u64) -> Vec<u8> {
    format!("key-{index:020}").into_bytes()
}

fn make_value(index: u64, len: usize) -> Vec<u8> {
    let mut value = vec![0; len];
    let prefix = format!("value-{index:020}:");
    let prefix = prefix.as_bytes();
    let prefix_len = prefix.len().min(value.len());
    value[..prefix_len].copy_from_slice(&prefix[..prefix_len]);

    for (offset, byte) in value.iter_mut().enumerate().skip(prefix_len) {
        *byte = ((index as usize + offset) % 251) as u8;
    }

    value
}

fn pseudo_random_index(op: u64, key_count: u64) -> u64 {
    if key_count == 0 {
        return 0;
    }

    op.wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407)
        % key_count
}

impl Options {
    fn parse() -> Self {
        let mut options = Self {
            workload: Workload::Write,
            data_dir: std::env::temp_dir().join("pocket-lsm-perf"),
            keys: 200_000,
            ops: 200_000,
            value_bytes: 256,
            memtable_threshold: 1024 * 1024,
            maximum_memtables: 4,
            read_percent: 50,
        };

        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--workload" => {
                    options.workload = Workload::parse(value(&args, index));
                    index += 2;
                }
                "--data-dir" => {
                    options.data_dir = PathBuf::from(value(&args, index));
                    index += 2;
                }
                "--keys" => {
                    options.keys = parse(value(&args, index), "--keys");
                    index += 2;
                }
                "--ops" => {
                    options.ops = parse(value(&args, index), "--ops");
                    index += 2;
                }
                "--value-bytes" => {
                    options.value_bytes = parse(value(&args, index), "--value-bytes");
                    index += 2;
                }
                "--memtable-threshold" => {
                    options.memtable_threshold = parse(value(&args, index), "--memtable-threshold");
                    index += 2;
                }
                "--maximum-memtables" => {
                    options.maximum_memtables = parse(value(&args, index), "--maximum-memtables");
                    index += 2;
                }
                "--read-percent" => {
                    options.read_percent = parse(value(&args, index), "--read-percent");
                    index += 2;
                }
                "--help" | "-h" => usage_and_exit(0),
                unknown => {
                    eprintln!("unknown argument: {unknown}");
                    usage_and_exit(2);
                }
            }
        }

        if options.read_percent > 100 {
            eprintln!("--read-percent must be <= 100");
            std::process::exit(2);
        }

        options
    }
}

impl Workload {
    fn parse(value: &str) -> Self {
        match value {
            "write" => Self::Write,
            "populate" => Self::Populate,
            "read" => Self::Read,
            "mixed" => Self::Mixed,
            _ => {
                eprintln!("unknown workload: {value}");
                usage_and_exit(2);
            }
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Populate => "populate",
            Self::Read => "read",
            Self::Mixed => "mixed",
        }
    }
}

fn value(args: &[String], index: usize) -> &str {
    args.get(index + 1)
        .unwrap_or_else(|| {
            eprintln!("missing value for {}", args[index]);
            usage_and_exit(2);
        })
        .as_str()
}

fn parse<T>(value: &str, flag: &str) -> T
where
    T: std::str::FromStr,
{
    value.parse().unwrap_or_else(|_| {
        eprintln!("invalid value for {flag}: {value}");
        usage_and_exit(2);
    })
}

fn usage_and_exit(code: i32) -> ! {
    eprintln!(
        "usage: perf_harness --workload write|populate|read|mixed --data-dir DIR [--keys N] [--ops N] [--value-bytes N] [--memtable-threshold N] [--maximum-memtables N] [--read-percent N]"
    );
    std::process::exit(code);
}
