use crate::{try_init, Service};
use toml::Value;

/// Initialize the logger using your project's TOML file.
///
/// This initializer includes your Cargo.toml file at compile time and extract the
/// service name and version at run time.
/// ## Usage
/// This is the basic form :
/// ```rust
/// use log::info;
///
/// stackdriver_logger::init_with_cargo!();
/// info!("Default path used for Cargo.toml : ../Cargo.toml");
/// ```
/// You can also specify the path if you need to :
/// ```rust
/// use log::info;
///
/// stackdriver_logger::init_with_cargo!("../Cargo.toml");
/// info!("Path was specified !");
/// ```
/// Note that the `init_with_cargo!` macro will include your `Cargo.toml` in the resulting binary.
/// If you don't want that, take a look at the other initializers.
#[macro_export]
macro_rules! init_with_cargo {
    ($e:expr) => {{
        let base = include_str!($e);
        $crate::macros::read_cargo(base);
    }};
    () => {{
        let base = include_str!("../Cargo.toml");
        $crate::macros::read_cargo(base);
    }};
}

#[doc(hidden)]
pub fn read_cargo(input: &str) {
    input
        .parse::<Value>()
        .ok()
        .and_then(|toml: Value| -> Option<()> {
            let service = Service {
                name: read_package_key(&toml, "name")?,
                version: read_package_key(&toml, "version")?,
            };

            try_init(Some(service), true).expect("Could not initialize stackdriver_logger");
            None
        });
}

fn read_package_key(toml: &Value, key: &str) -> Option<String> {
    let key = toml
        .get("package")?
        .as_table()?
        .get(key)?
        .as_str()?
        .to_owned();

    Some(key)
}
