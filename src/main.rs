use clap::Parser;
use hbmon::cli::{Cli, dispatch};

fn main() {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("hbmon: {}", e);
            std::process::exit(3);
        }
    }
}
