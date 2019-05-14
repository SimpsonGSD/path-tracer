extern crate path_tracer;

use path_tracer::Config;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let realtime = if args.len() > 1 {
        if args[1] == "-realtime" {
            true
        } else {
            false
        }
    } else {
        false
    };
    let config = Config::new(realtime);
    path_tracer::run(config);
}
