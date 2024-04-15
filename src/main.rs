use budget_ditto::Interfaces;

fn main() {
    // Get the name of the network interface from the command-line arguments
    let mut args: Vec<String> = std::env::args().collect();

    // Check if at least four arguments are provided
    if args.len() < 7 {
        eprintln!("Usage (give 4 interface names): {} <input> <obf_output> <obf_input> <output> <ipsrc> <ipdst>", args[0]);
        std::process::exit(1);
    }

    let pps = budget_ditto::get_env_var_f64("PPS").expect("Could not get PPS environment variable");

    let interfaces = Interfaces {
        // In reverse order since pop is lifo
        dst: budget_ditto::parse_ip(args.pop().unwrap_or_default()),
        src: budget_ditto::parse_ip(args.pop().unwrap_or_default()),
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

