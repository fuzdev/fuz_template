use std::io::{self, BufRead as _, IsTerminal as _, Write as _};

use crate::error::CliError;

/// Whether both stdin and stdout are terminals, enabling the wizard.
pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Prompts for a line of input; empty input (or EOF) selects `default`.
pub fn prompt(label: &str, default: Option<&str>) -> Result<String, CliError> {
    let mut stdout = io::stdout().lock();
    match default {
        Some(d) if !d.is_empty() => write!(stdout, "{label} [{d}]: ")?,
        _ => write!(stdout, "{label}: ")?,
    }
    stdout.flush()?;
    drop(stdout);
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim();
    Ok(if trimmed.is_empty() {
        default.unwrap_or("").to_owned()
    } else {
        trimmed.to_owned()
    })
}

/// Prompts for a yes/no answer; empty input (or EOF) selects `default`.
pub fn prompt_bool(label: &str, default: bool) -> Result<bool, CliError> {
    let suffix = if default { "[Y/n]" } else { "[y/N]" };
    loop {
        let mut stdout = io::stdout().lock();
        write!(stdout, "{label} {suffix}: ")?;
        stdout.flush()?;
        drop(stdout);
        let mut line = String::new();
        let bytes_read = io::stdin().lock().read_line(&mut line)?;
        let answer = line.trim().to_ascii_lowercase();
        if bytes_read == 0 || answer.is_empty() {
            return Ok(default);
        }
        match answer.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                let mut stdout = io::stdout().lock();
                writeln!(stdout, "please answer y or n")?;
            }
        }
    }
}
