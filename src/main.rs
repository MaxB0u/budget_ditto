extern crate pnet;

use std::env;
use std::process;




fn main() {
    // Get the name of the network interface from the command-line arguments
    let interface_name = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <interface>", env::args().next().unwrap());
        process::exit(1);
    });

    if let Err(e) = budget_ditto::run(&interface_name) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}