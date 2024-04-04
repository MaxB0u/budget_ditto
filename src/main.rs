use budget_ditto::Interfaces;
use std::env;

fn main() {
    // Get the name of the network interface from the command-line arguments
    let mut args: Vec<String> = std::env::args().collect();

    // Check if at least four arguments are provided
    if args.len() < 5 {
        eprintln!("Usage (give 4 interface names): {} <input> <obf_output> <obf_input> <output>", args[0]);
        std::process::exit(1);
    }

    let pps = match env::var("PPS") {
        Ok(pps_str) => {
            match pps_str.parse::<f64>() {
                Ok(pps) => {
                    pps
                },
                Err(e) => {
                    println!("Error parsing string: {}", e);
                    std::process::exit(1);
                }
            }
        },
        Err(e) => {
            eprintln!("Error getting env vairable PPS {}", e);
            std::process::exit(1);
        },
    };

    let interfaces = Interfaces {
        // In reverse order since pop is lifo
        output: args.pop().unwrap_or_default(),
        obfuscated_input: args.pop().unwrap_or_default(),
        obfuscated_output: args.pop().unwrap_or_default(),
        input: args.pop().unwrap_or_default(),   
        pps: pps, 
    };

    if let Err(e) = budget_ditto::run(interfaces) {
        eprintln!("Application error: {e}");
        std::process::exit(1);
    }
}

