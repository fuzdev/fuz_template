//! The starter CLI: replace this with your program.

mod error;

use std::process::ExitCode;

use argh::FromArgs;

use crate::error::CliError;

/// The `app_cli` CLI.
#[derive(FromArgs, Debug)]
struct TopLevel {
    /// print the version and exit
    #[argh(switch)]
    version: bool,

    /// who to greet
    #[argh(positional)]
    who: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let bin = args.first().map_or("app_cli", String::as_str);
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    // Parse explicitly rather than `argh::from_env`, which hard-exits `1` on any
    // parse failure — a usage error is `2` by shell convention.
    let top = match TopLevel::from_args(&[bin], &rest) {
        Ok(top) => top,
        Err(early_exit) => {
            return if early_exit.status.is_ok() {
                println!("{}", early_exit.output);
                ExitCode::SUCCESS
            } else {
                eprintln!("{}", early_exit.output);
                ExitCode::from(2)
            };
        }
    };
    match run(&top) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            if let Some(hint) = e.hint() {
                eprintln!("hint: {hint}");
            }
            ExitCode::from(e.exit_code())
        }
    }
}

fn run(top: &TopLevel) -> Result<(), CliError> {
    if top.version {
        println!("app_cli {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let who = top.who.as_deref().unwrap_or("world");
    if who.is_empty() {
        return Err(CliError::Usage("who must not be empty".to_owned()));
    }
    println!("hello {who}, from app_cli");
    Ok(())
}
