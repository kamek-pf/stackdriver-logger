# Stackdriver logger

A logger for Google's Stackdriver.\
In debug mode, we fall back back to [pretty-env-logger](https://github.com/seanmonstar/pretty-env-logger).\
In release mode, we output JSON formatted logs compatible with Stackdriver.

## Usage
```rust
#[macro_use]
extern crate log;
extern crate stackdriver_logger;

fn main() {
    stackdriver_logger::init();

    trace!("trace log");
    debug!("debug log");
    info!("info log");
    warn!("warn log");
    error!("error log");
}
```

## Behavior
Everything needed from Stackdriver is filled automatically, version numbers and service names are
read from your Cargo.toml


## License
Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

