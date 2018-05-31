extern crate log;
extern crate serde;
extern crate chrono;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use log::{Level, Log, Metadata, Record, SetLoggerError, STATIC_MAX_LEVEL};
use serde_json::Value;
use chrono::Utc;

pub struct StackdriverLogger;
static LOGGER: StackdriverLogger = StackdriverLogger;

const SVC_NAME: &str = env!("CARGO_PKG_NAME");
const SVC_VERSION: &str = env!("CARGO_PKG_VERSION");

impl Log for StackdriverLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= STATIC_MAX_LEVEL
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatted = format_record(record);
            println!("{}", formatted);
        }
    }

    fn flush(&self) {}
}

/// Log levels available in Stackdriver
#[derive(Debug, Serialize)]
pub enum StackdriverLogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

fn try_init() -> Result<(), SetLoggerError> {
    if cfg!(debug_assertions) {
        pretty_env_logger::init();
        Ok(())
    } else {
        log::set_max_level(STATIC_MAX_LEVEL);
        log::set_logger(&LOGGER)
    }
}

/// Initialize the logger.
/// For debug build, this falls back to pretty_env_logger.
/// For release build, we're using the json structure expected by Stackdriver.
pub fn init() {
    try_init().expect("Could not initialize stackdriver_logger");
}

fn format_record(record: &Record) -> Value {
    json!({
        "eventTime": Utc::now().to_rfc3339(),
        "serviceContext": {
            "service": SVC_NAME,
            "version": SVC_VERSION
        },
        "message": format!("{}", record.args()),
        "severity": map_level(&record.level()),
        "reportLocation": {
            "filePath": record.file(),
            "lineNumber": record.line(),
            "modulePath": record.module_path()
        }
    })
}

fn map_level(input: &Level) -> StackdriverLogLevel {
    match input {
        Level::Error => StackdriverLogLevel::Error,
        Level::Warn => StackdriverLogLevel::Warning,
        Level::Info => StackdriverLogLevel::Info,
        Level::Debug | Level::Trace => StackdriverLogLevel::Debug,
    }
}
