use std::{io::Write, path::PathBuf};

use argh::FromArgs;

#[derive(Debug, FromArgs)]
/// Clac intrepreter
struct Args {
    /// enable jit
    #[argh(switch, short = 'j')]
    jit: bool,

    /// input files
    #[argh(positional)]
    files: Vec<PathBuf>,
}

fn main() {
    let args: Args = argh::from_env();
    if args.jit {
        println!("=== JIT enabled ===");
    }

    // Check if files are accessible
    for file in &args.files {
        if !file.exists() {
            eprintln!("File {:?} does not exist", file);
            std::process::exit(1);
        }
    }

    let mut state = clacjit::State::new();

    for file in &args.files {
        let input = std::fs::read_to_string(file).unwrap();
        print!("Parsing file {:?}... ", file);
        state.parse(&input);
        println!("done");
    }

    // Eval
    print!("Evaluating...");
    let t0 = std::time::Instant::now();
    std::io::stdout().flush().unwrap();
    clacjit::eval(&mut state, args.jit);
    println!("Done in {:?}", t0.elapsed());

    // Simple REPL
    println!("Starting REPL");
    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        state.parse(&input);
        clacjit::eval(&mut state, args.jit);
    }
}
