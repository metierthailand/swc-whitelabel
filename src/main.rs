use std::process;

use wl_extractor::run::run;

fn main() {
    match run(Default::default()) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            process::exit(1)
        }
    }
}
