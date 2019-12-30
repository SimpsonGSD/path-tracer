extern crate path_tracer;

use path_tracer::Config;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::from_cmdline(&args);
    match path_tracer::run(config) {
        Err(e) => {
            if let Some(name) = e.name() {
                log::error!("Exit with {}: {}\n\nBACKTRACE:\n{}", name, e, e.backtrace());
            } else {
                log::error!(
                    "Exit with Unnamed Error: {}\n\nBACKTRACE: {}",
                    e,
                    e.backtrace()
                );
            }
        }
        _ => {}
    }
}
