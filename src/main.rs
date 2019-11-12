extern crate path_tracer;

use path_tracer::Config;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let realtime = if args.len() > 1 {
        if args[1] == "-offline" {
            false
        } else {
            true
        }
    } else {
        true
    };
    let config = Config::new(realtime);
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
