//! Command-line entry point for Ironshade.

use std::process::ExitCode;

const HELP: &str = "Ironshade shader compiler scaffold

Usage: ironshadec [--help | --version]

Shader compilation is unavailable in this scaffold.";

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    match (args.next().as_deref(), args.next()) {
        (None, _) => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        (Some(arg), None) if arg == "--help" || arg == "-h" => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        (Some(arg), None) if arg == "--version" || arg == "-V" => {
            println!("ironshadec {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("ironshadec: shader compilation is unavailable in this scaffold");
            ExitCode::FAILURE
        }
    }
}
