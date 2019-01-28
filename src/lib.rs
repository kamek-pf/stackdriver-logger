// #![doc(include = "../README.md")]

#[macro_use]
extern crate serde_json;

use chrono::Utc;
use env_logger::Builder;
use log::{Level, Record, SetLoggerError};
use serde_json::Value;
use std::env;
use std::fmt;

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

fn try_init(service: Option<Service>, report_location: bool) -> Result<(), SetLoggerError> {
    if cfg!(debug_assertions) {
        pretty_env_logger::try_init()
    } else {
        let mut builder = formatted_builder(service, report_location);

        if let Ok(s) = ::std::env::var("RUST_LOG") {
            builder.parse(&s);
        }

        builder.try_init()
    }
}

/// Initialize the logger.
/// For debug build, this falls back to pretty_env_logger.
/// For release build, we're using the json structure expected by Stackdriver (without service and
/// with report location).
pub fn init() {
    try_init(None, true).expect("Could not initialize stackdriver_logger");
}

/// Initialize the logger.
/// For debug build, this falls back to pretty_env_logger.
/// For release build, we're using the json structure expected by Stackdriver.
pub fn init_with(service: Option<Service>, report_location: bool) {
    try_init(service, report_location).expect("Could not initialize stackdriver_logger");
}

/// Returns a `env_logger::Builder` for further customization.
///
/// This method will return a stackdriver JSON formatted `env_logger::Builder`
/// for further customization. Refer to env_logger::Build crate documentation
/// for further details and usage.
pub fn formatted_builder(service: Option<Service>, report_location: bool) -> Builder {
    use std::io::Write;

    let mut builder = Builder::new();

    builder.format(move |f, record| {
        writeln!(
            f,
            "{}",
            format_record(record, service.as_ref(), report_location)
        )
    });

    builder
}

fn map_level(input: &Level) -> StackdriverLogLevel {
    match input {
        Level::Error => StackdriverLogLevel::Error,
        Level::Warn => StackdriverLogLevel::Warning,
        Level::Info => StackdriverLogLevel::Info,
        Level::Debug | Level::Trace => StackdriverLogLevel::Debug,
    }
}

#[derive(Clone)]
pub struct Service {
    name: String,
    version: String,
}

impl Service {
    pub fn from_env() -> Option<Service> {
        let name = env::var("SERVICE_NAME")
            .or(env::var("CARGO_PKG_NAME"))
            .unwrap_or("".to_owned());

        let version = env::var("SERVICE_VERSION")
            .or(env::var("CARGO_PKG_VERSION"))
            .unwrap_or("".to_owned());

        if name.is_empty() && version.is_empty() {
            return None;
        }

        Some(Service { name, version })
    }
}

fn format_record(record: &Record<'_>, service: Option<&Service>, report_location: bool) -> Value {
    let message = match record.level() {
        Level::Error => format!(
            "{} \n at {}:{}",
            record.args(),
            record.file().unwrap_or("unknown_file"),
            record.line().unwrap_or(0)
        ),
        _ => format!("{}", record.args()),
    };

    let mut value = json!({
        "eventTime": Utc::now().to_rfc3339(),
        "message": message,
        "severity": map_level(&record.level()).to_string(),
    });
    if let Some(service) = service {
        value.as_object_mut().unwrap().insert(
            "serviceContext".to_string(),
            json!({
                "service": service.name,
                "version": service.version
            }),
        );
    }
    if report_location {
        value.as_object_mut().unwrap().insert(
            "reportLocation".to_string(),
            json!({
                "filePath": record.file(),
                "modulePath": record.module_path(),
                "lineNumber": record.line(),
            }),
        );
    }
    value
}
