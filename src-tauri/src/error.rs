use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// A PowerShell or pnputil child exited non-zero.
    Command {
        what: &'static str,
        detail: String,
    },
    Parse(String),
    Timeout(&'static str),
    Cancelled,
    /// Refused by a safety check before anything changed.
    Denied(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O: {e}"),
            Error::Command { what, detail } if detail.trim().is_empty() => {
                write!(f, "{what} falhou sem detalhes")
            }
            Error::Command { what, detail } => write!(f, "{what} falhou: {detail}"),
            Error::Parse(m) => write!(f, "resposta inesperada do Windows ({m})"),
            Error::Timeout(what) => write!(f, "{what} excedeu o tempo limite"),
            Error::Cancelled => write!(f, "cancelado"),
            Error::Denied(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_with_no_output_does_not_end_in_a_bare_colon() {
        let e = Error::Command {
            what: "pnputil",
            detail: "  ".into(),
        };
        assert_eq!(e.to_string(), "pnputil falhou sem detalhes");
    }

    #[test]
    fn a_command_with_output_keeps_its_detail() {
        let e = Error::Command {
            what: "pnputil",
            detail: "acesso negado".into(),
        };
        assert_eq!(e.to_string(), "pnputil falhou: acesso negado");
    }
}
