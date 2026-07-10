use thiserror::Error;

/// Errors surfaced by the `app_cli` CLI.
///
/// Exit-code dialect: `0` success; `2` = the caller must change something
/// local (arguments, config) before retrying; `1` = everything else.
#[derive(Debug, Error)]
pub enum CliError {
    /// The invocation itself is wrong.
    #[error("{0}")]
    Usage(String),
}

impl CliError {
    /// Maps the error to its stable exit code.
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
        }
    }

    /// An optional remediation hint printed under the error.
    pub const fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Usage(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(CliError::Usage(String::new()).exit_code(), 2);
    }
}
