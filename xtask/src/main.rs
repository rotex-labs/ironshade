//! Repository maintenance entry point.

mod conformance;
mod difftest;
mod layering;
mod operations;
mod oracles;

use std::path::Path;
use std::process::ExitCode;

use operations::Result;

const HELP: &str = "Usage: cargo xtask <command>

check-layering                                  Validate workspace dependency boundaries
fetch-oracles                                   Provision pinned oracle tools
difftest --oracle-only <fixtures> [--update]    Check or regenerate Nova snapshots
conformance --dry-run                          Audit and classify the Slang corpus
conformance --audit                            Audit Slang corpus licenses";

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(command) = args.first() else {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    };
    let result: Result<()> = match args.as_slice() {
        [arg] if arg == "--help" || arg == "-h" => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        [arg] if arg == "check-layering" => layering::check().map_err(Into::into).map(|()| {
            println!("Ironshade dependency layering passed.");
        }),
        [arg] if arg == "fetch-oracles" => oracles::fetch(),
        [arg, mode, path, options @ ..]
            if arg == "difftest"
                && mode == "--oracle-only"
                && (options.is_empty() || (options.len() == 1 && options[0] == "--update")) =>
        {
            difftest::run(Path::new(path), !options.is_empty())
        }
        [arg, mode] if arg == "conformance" && (mode == "--audit" || mode == "--dry-run") => {
            conformance::run(mode == "--dry-run")
        }
        _ => {
            eprintln!("{HELP}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: {error}", command.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
