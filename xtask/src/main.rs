//! Repository maintenance entry point.

mod layering;

use std::process::ExitCode;

const HELP: &str = "Usage: cargo xtask [--help | check-layering]

check-layering   Validate workspace dependency boundaries";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        [arg] if arg == "--help" || arg == "-h" => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        [arg] if arg == "check-layering" => match layering::check() {
            Ok(()) => {
                println!("Ironshade dependency layering passed.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("check-layering: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{HELP}");
            ExitCode::from(2)
        }
    }
}
