// #![doc(include = "../README.md")]

extern crate log;
extern crate chrono;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_json;

use std::fmt;
use std::env;

use log::{Level, Log, Metadata, Record, SetLoggerError, STATIC_MAX_LEVEL};
use serde_json::Value;
use chrono::Utc;

struct StackdriverLogger {
    service_name: String,
    service_version: String,
}

impl StackdriverLogger {
    fn format_record(&self, record: &Record) -> Value {
        json!({
            "eventTime": Utc::now().to_rfc3339(),
            "serviceContext": {
                "service": self.service_name,
                "version": self.service_version
            },
            "message": format!("{}", record.args()),
            "severity": map_level(&record.level()).to_string(),
            "reportLocation": {
                "filePath": record.file(),
                "lineNumber": record.line(),
                "modulePath": record.module_path()
            }
        })
    }
}

impl Log for StackdriverLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= STATIC_MAX_LEVEL
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatted = self.format_record(record);

            if record.metadata().level() == Level::Error {
                eprintln!("{}", formatted);
            } else {
                println!("{}", formatted);
            }
        }
    }

    fn flush(&self) {}
}

/// Log levels available in Stackdriver
#[derive(Debug)]
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

impl fmt::Display for StackdriverLogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StackdriverLogLevel::Debug => write!(f, "DEBUG"),
            StackdriverLogLevel::Info => write!(f, "INFO"),
            StackdriverLogLevel::Notice => write!(f, "NOTICE"),
            StackdriverLogLevel::Warning => write!(f, "WARNING"),
            StackdriverLogLevel::Error => write!(f, "ERROR"),
            StackdriverLogLevel::Critical => write!(f, "CRITICAL"),
            StackdriverLogLevel::Alert => write!(f, "ALERT"),
            StackdriverLogLevel::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

fn try_init() -> Result<(), SetLoggerError> {
    if cfg!(debug_assertions) {
        pretty_env_logger::init();
        Ok(())
    } else {
        let service_name = env::var("SERVICE_NAME")
            .or(env::var("CARGO_PKG_NAME"))
            .unwrap_or("".to_owned());

        let service_version = env::var("SERVICE_VERSION")
            .or(env::var("CARGO_PKG_VERSION"))
            .unwrap_or("".to_owned());

        let logger = StackdriverLogger {
            service_name,
            service_version,
        };

        log::set_max_level(STATIC_MAX_LEVEL);
        log::set_boxed_logger(Box::new(logger))
    }
}

/// Initialize the logger.
/// For debug build, this falls back to pretty_env_logger.
/// For release build, we're using the json structure expected by Stackdriver.
pub fn init() {
    try_init().expect("Could not initialize stackdriver_logger");
}

fn map_level(input: &Level) -> StackdriverLogLevel {
    match input {
        Level::Error => StackdriverLogLevel::Error,
        Level::Warn => StackdriverLogLevel::Warning,
        Level::Info => StackdriverLogLevel::Info,
        Level::Debug | Level::Trace => StackdriverLogLevel::Debug,
    }
}
