// #![doc(include = "../README.md")]

use std::{env, fmt};

use chrono::Utc;
use env_logger::Builder;
use log::{Level, Record, SetLoggerError};
use serde_json::{json, Value};

#[cfg(feature = "cargo")]
#[doc(hidden)]
#[macro_use]
pub mod macros;

/// Log levels available in Stackdriver
#[derive(Debug)]
enum StackdriverLogLevel {
    Debug,
    Info,
    Warning,
    Error,
    // Notice,
    // Critical,
    // Alert,
    // Emergency,
}

/// Parameters expected by the logger, used for manual initialization.
#[derive(Clone)]
pub struct Service {
    /// Name of your service as it will be reported by Stackdriver
    pub name: String,

    /// Version of your service as it will be reported by Stackdriver
    pub version: String,
}

/// Basic initializer, expects SERVICE_NAME and SERVICE_VERSION env variables
/// to be defined, otherwise you won't have much context available in Stackdriver.
/// ## Usage
/// ```rust
/// use log::info;
///
/// fn main() {
///     stackdriver_logger::init();
///     info!("Make sur you don't forget the env variables !");
/// }
/// ```
pub fn init() {
    try_init(None, true).expect("Could not initialize stackdriver_logger");
}

/// Initialize the logger manually.
/// ## Usage
/// With everything manually specified :
/// ```rust
/// use log::info;
/// use stackdriver_logger::Service;
///
/// fn main() {
///     let params = Service {
///         name: "My Service".to_owned(),
///         version: "2.3.1".to_owned(),
///     };
///
///     stackdriver_logger::init_with(Some(params), true);
///     info!("We're all set here !");
/// }
/// ```
/// You can also pass a `None` instead of `Some(Service{ ... })` and define the `SERVICE_NAME`
/// and `SERVICE_VERSION` env variables :
/// ```rust
/// use log::info;
///
/// fn main() {
///     stackdriver_logger::init_with(None, false);
///     info!("Make sur you don't forget the env variables !");
/// }
/// ```
pub fn init_with(service: Option<Service>, report_location: bool) {
    try_init(service, report_location).expect("Could not initialize stackdriver_logger");
}

pub(crate) fn try_init(
    service: Option<Service>,
    report_location: bool,
) -> Result<(), SetLoggerError> {
    if cfg!(debug_assertions) {
        pretty_env_logger::try_init()
    } else {
        let mut builder = formatted_builder(service, report_location);

        if let Ok(s) = ::std::env::var("RUST_LOG") {
            builder.parse_filters(&s);
        }

        builder.try_init()
    }
}

impl fmt::Display for StackdriverLogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StackdriverLogLevel::Debug => write!(f, "DEBUG"),
            StackdriverLogLevel::Info => write!(f, "INFO"),
            StackdriverLogLevel::Warning => write!(f, "WARNING"),
            StackdriverLogLevel::Error => write!(f, "ERROR"),
            // StackdriverLogLevel::Notice => write!(f, "NOTICE"),
            // StackdriverLogLevel::Critical => write!(f, "CRITICAL"),
            // StackdriverLogLevel::Alert => write!(f, "ALERT"),
            // StackdriverLogLevel::Emergency => write!(f, "EMERGENCY"),
        }
    }
}

impl Service {
    pub fn from_env() -> Option<Service> {
        let name = env::var("SERVICE_NAME")
            .or_else(|_| env::var("CARGO_PKG_NAME"))
            .unwrap_or_else(|_| "".to_owned());

        let version = env::var("SERVICE_VERSION")
            .or_else(|_| env::var("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_| "".to_owned());

        if name.is_empty() && version.is_empty() {
            return None;
        }

        Some(Service { name, version })
    }
}

fn formatted_builder(service: Option<Service>, report_location: bool) -> Builder {
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

fn map_level(input: Level) -> StackdriverLogLevel {
    match input {
        Level::Error => StackdriverLogLevel::Error,
        Level::Warn => StackdriverLogLevel::Warning,
        Level::Info => StackdriverLogLevel::Info,
        Level::Debug | Level::Trace => StackdriverLogLevel::Debug,
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
        "severity": map_level(record.level()).to_string(),
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
