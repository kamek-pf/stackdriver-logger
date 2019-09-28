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

// Wrap Level from the log crate so we can implement standard traits for it
struct LogLevel(Level);

/// Parameters expected by the logger, used for manual initialization.
#[derive(Clone)]
pub struct Service {
    /// Name of your service as it will be reported by Stackdriver
    pub name: String,

    /// Version of your service as it will be reported by Stackdriver
    pub version: String,
}

impl Service {
    pub fn from_env() -> Option<Service> {
        let name = env::var("SERVICE_NAME")
            .or_else(|_| env::var("CARGO_PKG_NAME"))
            .unwrap_or_else(|_| String::new());

        let version = env::var("SERVICE_VERSION")
            .or_else(|_| env::var("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_| String::new());

        if name.is_empty() && version.is_empty() {
            return None;
        }

        Some(Service { name, version })
    }
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

// Initialize the logger, defaults to pretty_env_logger in debug mode
pub(crate) fn try_init(
    service: Option<Service>,
    report_location: bool,
) -> Result<(), SetLoggerError> {
    if cfg!(debug_assertions) {
        pretty_env_logger::try_init()
    } else {
        use std::io::Write;
        let mut builder = Builder::new();
        builder.format(move |f, record| {
            writeln!(
                f,
                "{}",
                format_record(record, service.as_ref(), report_location)
            )
        });

        if let Ok(s) = ::std::env::var("RUST_LOG") {
            builder.parse_filters(&s);
        }

        builder.try_init()
    }
}

// Format log level for Stackdriver
impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel(l) if l == &Level::Error => write!(f, "ERROR"),
            LogLevel(l) if l == &Level::Warn => write!(f, "WARNING"),
            LogLevel(l) if l == &Level::Info => write!(f, "INFO"),

            // Debug and Trace are caught here. Stackdriver doesn't have Trace, we map it to Debug instead
            LogLevel(_) => write!(f, "DEBUG"),
        }
    }
}

// Message structure is documented here: https://cloud.google.com/error-reporting/docs/formatting-error-messages
fn format_record(record: &Record<'_>, service: Option<&Service>, report_location: bool) -> Value {
    json!({
        "eventTime": Utc::now().to_rfc3339(),
        "severity": LogLevel(record.level()).to_string(),

        // Error messages also have a pseudo stack trace
        "message": match record.level() {
            Level::Error => format!(
                "{} \n at {}:{}",
                record.args(),
                record.file().unwrap_or("unknown_file"),
                record.line().unwrap_or(0)
            ),
            _ => format!("{}", record.args()),
        },

        // Service context may or may not be defined
        "serviceContext": service.map(|s| json!({
                "service": s.name,
                "version": s.version
            }))
            .unwrap_or_else(|| json!({
                "service": "unknown_service"
            })),

        // Report location may or may not be available
        "reportLocation": if report_location {
            json!({
                "filePath": record.file(),
                "modulePath": record.module_path(),
                "lineNumber": record.line(),
            })
        } else {
            Value::Null
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_formatter() {
        let svc = Service {
            name: String::from("test"),
            version: String::from("0.0.0"),
        };

        let record = Record::builder()
            .args(format_args!("Info!"))
            .level(Level::Info)
            .target("test_app")
            .file(Some("my_file.rs"))
            .line(Some(1337))
            .module_path(Some("my_module"))
            .build();

        let mut output = format_record(&record, Some(&svc), false);
        let expected = include_str!("../test_snapshots/info_svc.json");
        let expected: Value = serde_json::from_str(expected).unwrap();

        // Make sure eventTime is set then overwrite generated timestamp with a known value
        assert!(output["eventTime"].as_str().is_some());
        *output.get_mut("eventTime").unwrap() = json!("2019-09-28T04:00:00.000000000+00:00");
        assert_eq!(output, expected);
    }

    #[test]
    fn error_formatter() {
        let svc = Service {
            name: String::from("test"),
            version: String::from("0.0.0"),
        };

        let record = Record::builder()
            .args(format_args!("Error!"))
            .level(Level::Error)
            .target("test_app")
            .file(Some("my_file.rs"))
            .line(Some(1337))
            .module_path(Some("my_module"))
            .build();

        let mut output = format_record(&record, None, false);
        let expected = include_str!("../test_snapshots/no_scv_no_loc.json");
        let expected: Value = serde_json::from_str(expected).unwrap();
        assert!(output["eventTime"].as_str().is_some());
        *output.get_mut("eventTime").unwrap() = json!("2019-09-28T04:00:00.000000000+00:00");
        assert_eq!(output, expected);

        let mut output = format_record(&record, Some(&svc), true);
        let expected = include_str!("../test_snapshots/svc_and_loc.json");
        let expected: Value = serde_json::from_str(expected).unwrap();
        assert!(output["eventTime"].as_str().is_some());
        *output.get_mut("eventTime").unwrap() = json!("2019-09-28T04:00:00.000000000+00:00");
        assert_eq!(output, expected);
    }
}
