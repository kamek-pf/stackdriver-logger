// #![doc(include = "../README.md")]

extern crate chrono;
extern crate log;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_json;

use std::env;
use std::fmt;

use chrono::Utc;
use log::{Level, Log, Metadata, Record, SetLoggerError, STATIC_MAX_LEVEL};
use serde_json::Value;

struct StackdriverLogger {
    service_name: String,
    service_version: String,
    ignored_paths: Vec<String>,
}

impl StackdriverLogger {
    fn format_record(&self, record: &Record) -> Value {
        let message = match record.level() {
            Level::Error => format!(
                "{} \n at {}:{}",
                record.args(),
                record.file().unwrap_or("unknown_file"),
                record.line().unwrap_or(0)
            ),
            _ => format!("{}", record.args()),
        };

        json!({
            "eventTime": Utc::now().to_rfc3339(),
            "serviceContext": {
                "service": self.service_name,
                "version": self.service_version
            },
            "message": message,
            "severity": map_level(&record.level()).to_string(),
            "reportLocation": {
                "filePath": record.file(),
                "modulePath": record.module_path(),
                "lineNumber": record.line(),
            }
        })
    }
}

impl Log for StackdriverLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let has_level = metadata.level() <= STATIC_MAX_LEVEL;
        let is_allowed = !self.ignored_paths.iter().any(|e| e == metadata.target());

        has_level && is_allowed
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatted = self.format_record(record);

            if record.level() == Level::Error {
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
            ignored_paths: get_ignored_paths(env::var("RUST_LOG_IGNORE").ok()),
        };

        log::set_max_level(STATIC_MAX_LEVEL);
        log::set_boxed_logger(Box::new(logger))
    }
}

fn get_ignored_paths(paths: Option<String>) -> Vec<String> {
    match paths {
        None => vec![],
        Some(value) => value
            .split(',')
            .filter_map(|value| match value {
                "" => None,
                other => Some(other.to_owned()),
            })
            .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ignored_() {
        let input = None;
        let paths = get_ignored_paths(input);
        let expected: Vec<String> = vec![];
        assert_eq!(paths, expected);

        let input = Some("module".to_owned());
        let paths = get_ignored_paths(input);
        assert_eq!(paths, vec!["module"]);

        let input = Some("some::module".to_owned());
        let paths = get_ignored_paths(input);
        assert_eq!(paths, vec!["some::module"]);

        let input = Some("some::module,and_another".to_owned());
        let paths = get_ignored_paths(input);
        assert_eq!(paths, vec!["some::module", "and_another"]);

        let input = Some("some::module,and_another,plus::something_else,".to_owned());
        let paths = get_ignored_paths(input);
        assert_eq!(
            paths,
            vec!["some::module", "and_another", "plus::something_else"]
        );

        let input = Some("some::module,and_another,with::trailing::comma,".to_owned());
        let paths = get_ignored_paths(input);
        assert_eq!(
            paths,
            vec!["some::module", "and_another", "with::trailing::comma"]
        );
    }
}
