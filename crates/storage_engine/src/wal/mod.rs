mod log;
mod logger;
mod record;

pub use log::WriteAheadLog;
pub use logger::WriteAheadLogger;
pub use record::WalRecord;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}.wal"))
    }

    #[test]
    fn record_encode_decode_round_trip() {
        let path = temp_path("wal-round-trip");
        let mut wal = WriteAheadLog::new(&path);

        wal.append(WalRecord::put(1, b"a".to_vec(), b"one".to_vec()))
            .unwrap();
        wal.append(WalRecord::delete(2, b"b".to_vec())).unwrap();

        let records = WriteAheadLog::replay(&path).unwrap();

        assert_eq!(records, wal.records());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn truncate_removes_records_and_file_contents() {
        let path = temp_path("wal-truncate");
        let mut wal = WriteAheadLog::new(&path);
        wal.append(WalRecord::put(1, b"a".to_vec(), b"one".to_vec()))
            .unwrap();

        wal.truncate().unwrap();

        assert!(wal.records().is_empty());
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn replay_rejects_checksum_mismatch() {
        let path = temp_path("wal-checksum");
        let mut wal = WriteAheadLog::new(&path);
        wal.append(WalRecord::put(1, b"a".to_vec(), b"one".to_vec()))
            .unwrap();

        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        std::fs::write(&path, bytes).unwrap();

        assert!(WriteAheadLog::replay(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn logger_uses_queue_for_segments() {
        let dir = std::env::temp_dir().join(format!(
            "wal-queue-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut logger = WriteAheadLogger::new(dir.clone()).unwrap();

        assert!(logger.current_mut().is_some());
        logger.new_wal();
        assert_eq!(logger.len(), 2);
        assert!(logger.pop_oldest().is_some());
        assert_eq!(logger.len(), 1);

        std::fs::remove_dir_all(dir).unwrap();
    }
}
