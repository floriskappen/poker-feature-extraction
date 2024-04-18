use log::{Level, Log, Metadata, Record, SetLoggerError};

struct TimestampLogger;

impl Log for TimestampLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            println!("{} - {}: {}", timestamp, record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() -> Result<(), SetLoggerError> {
    log::set_max_level(Level::Info.to_level_filter());
    log::set_logger(&TimestampLogger)
}