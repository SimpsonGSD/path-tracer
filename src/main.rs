extern crate path_tracer;

use path_tracer::Config;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut max_depth = 10;
    let mut realtime = true;
    if args.len() > 1 {
        if args[1] == "-offline" {
            realtime = false;
            max_depth = 100;
        }
    }
    let config = Config::new(realtime, max_depth);
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
