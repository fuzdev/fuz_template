use std::io::{self, BufRead as _, IsTerminal as _, Write as _};

use crate::error::CliError;

/// Whether both stdin and stdout are terminals, enabling the wizard.
pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Maps a terminal io failure to `CliError::Terminal` — the only place raw
/// io errors become that variant.
fn terminal<T>(result: io::Result<T>) -> Result<T, CliError> {
    result.map_err(CliError::Terminal)
}

/// Prompts for a line of input; empty input (or EOF) selects `default`.
pub fn prompt(label: &str, default: Option<&str>) -> Result<String, CliError> {
    Ok(prompt_raw(label, default)?.0)
}

/// Prompts until `validate` accepts the input, echoing the validation error
/// between attempts. On EOF the error surfaces instead of looping forever.
pub fn prompt_validated(
    label: &str,
    default: Option<&str>,
    validate: impl Fn(&str) -> Result<(), CliError>,
) -> Result<String, CliError> {
    loop {
        let (input, eof) = prompt_raw(label, default)?;
        match validate(&input) {
            Ok(()) => return Ok(input),
            Err(e) if eof => return Err(e),
            Err(e) => println!("{e}"),
        }
    }
}

/// Prompts for a line; returns the resolved value and whether stdin hit EOF.
fn prompt_raw(label: &str, default: Option<&str>) -> Result<(String, bool), CliError> {
    let mut stdout = io::stdout().lock();
    match default {
        Some(d) if !d.is_empty() => terminal(write!(stdout, "{label} [{d}]: "))?,
        _ => terminal(write!(stdout, "{label}: "))?,
    }
    terminal(stdout.flush())?;
    drop(stdout);
    let mut line = String::new();
    let bytes_read = terminal(io::stdin().lock().read_line(&mut line))?;
    let trimmed = line.trim();
    let value = if trimmed.is_empty() {
        default.unwrap_or("").to_owned()
    } else {
        trimmed.to_owned()
    };
    Ok((value, bytes_read == 0))
}

/// Prompts for a yes/no answer; empty input (or EOF) selects `default`.
pub fn prompt_bool(label: &str, default: bool) -> Result<bool, CliError> {
    let suffix = if default { "[Y/n]" } else { "[y/N]" };
    loop {
        let mut stdout = io::stdout().lock();
        terminal(write!(stdout, "{label} {suffix}: "))?;
        terminal(stdout.flush())?;
        drop(stdout);
        let mut line = String::new();
        let bytes_read = terminal(io::stdin().lock().read_line(&mut line))?;
        let answer = line.trim().to_ascii_lowercase();
        if bytes_read == 0 || answer.is_empty() {
            return Ok(default);
        }
        match answer.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                let mut stdout = io::stdout().lock();
                terminal(writeln!(stdout, "please answer y or n"))?;
            }
        }
    }
}
