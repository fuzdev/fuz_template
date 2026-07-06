use std::path::PathBuf;

use thiserror::Error;

/// Errors surfaced by molt.
///
/// Exit-code dialect: `0` success; `2` = the caller must change something
/// local (arguments, git state) before retrying; `1` = everything else.
#[derive(Debug, Error)]
pub enum CliError {
    /// The invocation itself is wrong (bad flag values, missing required args).
    #[error("{0}")]
    Usage(String),

    /// The environment isn't ready (not a git repo, dirty tree, wrong directory).
    #[error("{message}")]
    Precondition {
        message: String,
        hint: Option<&'static str>,
    },

    /// The template's files no longer match molt's anchors — a template bug,
    /// not something the caller can fix.
    #[error("template drift detected:\n{0}")]
    Drift(String),

    /// Filesystem failure while planning or applying.
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Terminal input/output failed mid-wizard.
    #[error("terminal io failed: {0}")]
    Terminal(#[from] std::io::Error),
}

impl CliError {
    pub fn precondition(message: impl Into<String>, hint: Option<&'static str>) -> Self {
        Self::Precondition {
            message: message.into(),
            hint,
        }
    }

    /// Maps the error to its stable exit code.
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::Precondition { .. } => 2,
            Self::Drift(_) | Self::Io { .. } | Self::Terminal(_) => 1,
        }
    }

    /// An optional remediation hint printed under the error.
    pub const fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Precondition { hint, .. } => *hint,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_stable() {
        assert_eq!(CliError::Usage(String::new()).exit_code(), 2);
        assert_eq!(CliError::precondition("x", None).exit_code(), 2);
        assert_eq!(CliError::Drift(String::new()).exit_code(), 1);
        assert_eq!(
            CliError::Io {
                path: PathBuf::new(),
                source: std::io::Error::other("x"),
            }
            .exit_code(),
            1
        );
    }
}
