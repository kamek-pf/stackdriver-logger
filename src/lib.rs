// #![doc(include = "../README.md")]
#![forbid(unsafe_code)]

use std::{env, fmt};

use log::{Level, SetLoggerError};

#[cfg(any(test, not(all(feature = "pretty_env_logger", debug_assertions))))]
use serde_json::{json, Value};

#[cfg(feature = "cargo")]
#[doc(hidden)]
#[macro_use]
pub mod macros;

#[cfg(feature = "customfields")]
use log::kv;

#[cfg(feature = "customfields")]
use std::collections::HashMap;

// Wrap Level from the log crate so we can implement standard traits for it
struct LogLevel(Level);

// Wrap a Hashmap so we can implement log::kv traits for structured logging of custom fields
// See https://cloud.google.com/logging/docs/view/overview#custom-fields
#[cfg(feature = "customfields")]
struct CustomFields<'kvs>(HashMap<kv::Key<'kvs>, kv::Value<'kvs>>);

#[cfg(feature = "customfields")]
impl<'kvs> CustomFields<'kvs> {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn inner(&self) -> &HashMap<kv::Key, kv::Value> {
        &self.0
    }
}

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
/// stackdriver_logger::init();
/// info!("Make sur you don't forget the env variables !");
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
/// let params = Service {
///     name: "My Service".to_owned(),
///     version: "2.3.1".to_owned(),
/// };
///
/// stackdriver_logger::init_with(Some(params), true);
/// info!("We're all set here !");
/// ```
/// You can also pass a `None` instead of `Some(Service{ ... })` and define the `SERVICE_NAME`
/// and `SERVICE_VERSION` env variables :
/// ```rust
/// use log::info;
///
/// stackdriver_logger::init_with(None, false);
/// info!("Make sur you don't forget the env variables !");
/// ```
pub fn init_with(service: Option<Service>, report_location: bool) {
    try_init(service, report_location).expect("Could not initialize stackdriver_logger");
}

// Initialize the logger, defaults to pretty_env_logger in debug mode
// Allow unused variables for convenience when toggling feature flags
#[allow(unused_variables)]
pub(crate) fn try_init(
    service: Option<Service>,
    report_location: bool,
) -> Result<(), SetLoggerError> {
    #[cfg(all(feature = "pretty_env_logger", debug_assertions))]
    {
        #[cfg(feature = "customfields")]
        {
            use std::io::Write;
            let mut builder = env_logger::Builder::new();
            builder.format(move |f, record| {
                writeln!(
                    f,
                    "{}",
                    format_record_pretty(record)
                )
            });
        }

        pretty_env_logger::try_init()
    }

    #[cfg(not(all(feature = "pretty_env_logger", debug_assertions)))]
    {
        use std::io::Write;
        let mut builder = env_logger::Builder::new();
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

#[cfg(feature = "customfields")]
impl<'kvs> kv::Visitor<'kvs> for CustomFields<'kvs> {
    fn visit_pair(&mut self, key: kv::Key<'kvs>, value: kv::Value<'kvs>) -> Result<(), kv::Error> {
        self.0.insert(key, value);
        Ok(())
    }
}

// Message structure is documented here: https://cloud.google.com/error-reporting/docs/formatting-error-messages
#[cfg(any(test, not(all(feature = "pretty_env_logger", debug_assertions))))]
fn format_record(
    record: &log::Record<'_>,
    service: Option<&Service>,
    report_location: bool,
) -> Value {
    let json_payload = json!({
        "eventTime": chrono::Utc::now().to_rfc3339(),
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
    });

    #[cfg(not(feature = "customfields"))]
    return json_payload;

    #[cfg(feature = "customfields")]
    {
        let mut json_payload = json_payload;
        let mut custom_fields = CustomFields::new();
        if let Ok(_) = record.key_values().visit(&mut custom_fields) {
            for (key, val) in custom_fields.inner().iter() {
                json_payload[key.as_str()] = Value::String(val.to_string());
            }
        }
        return json_payload;
    }
}

#[cfg(all(feature = "pretty_env_logger", feature = "customfields", debug_assertions))]
fn format_record_pretty(
    record: &log::Record<'_>
) -> String {    
    let mut message = format!("{}", record.args());
    let mut custom_fields = CustomFields::new();
    let mut kv_message_parts = vec![];
    if let Ok(_) = record.key_values().visit(&mut custom_fields) {
        for (key, val) in custom_fields.inner().iter() {
            kv_message_parts.push(format!("{}={}", key, val));
        }
    }

    if kv_message_parts.len() > 0 {
        kv_message_parts.sort();
        message = format!("{} {}", message, kv_message_parts.join(", "))
    }

    message
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

        let record = log::Record::builder()
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

        let record = log::Record::builder()
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

    #[test]
    #[cfg(feature = "customfields")]
    fn custom_fields_formatter() {
        let svc = Service {
            name: String::from("test"),
            version: String::from("0.0.0"),
        };

        let mut map = std::collections::HashMap::new();
        map.insert("a", "a value");
        map.insert("b", "b value");

        let record = log::Record::builder()
            .args(format_args!("Info!"))
            .level(Level::Info)
            .target("test_app")
            .file(Some("my_file.rs"))
            .line(Some(1337))
            .module_path(Some("my_module"))
            .key_values(&mut map)
            .build();

        let mut output = format_record(&record, Some(&svc), false);
        let expected = include_str!("../test_snapshots/custom_fields.json");
        let expected: Value = serde_json::from_str(expected).unwrap();

        // Make sure eventTime is set then overwrite generated timestamp with a known value
        assert!(output["eventTime"].as_str().is_some());
        *output.get_mut("eventTime").unwrap() = json!("2019-09-28T04:00:00.000000000+00:00");
        assert_eq!(output, expected);
    }

    #[test]
    #[cfg(feature = "customfields")]
    fn custom_fields_formatter_pretty() {
        let mut map = std::collections::HashMap::new();
        map.insert("a", "a value");
        map.insert("b", "b value");

        let record = log::Record::builder()
            .args(format_args!("Info!"))
            .level(Level::Info)
            .target("test_app")
            .file(Some("my_file.rs"))
            .line(Some(1337))
            .module_path(Some("my_module"))
            .key_values(&mut map)
            .build();

        let output = format_record_pretty(&record);
        let expected = "Info! a=a value, b=b value";

        assert_eq!(output, expected);
    }
}
