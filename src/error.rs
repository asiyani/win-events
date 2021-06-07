use std::fmt;
use std::io;

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Event => write!(f, "{}", self.message),
            ErrorKind::Subscription => write!(f, "{}", self.message),
            _ => write!(f, "{}", self.kind),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd)]
pub enum ErrorKind {
    Event,
    Subscription,
    NoMoreLogs,
    // XmlParseError,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err_msg = match *self {
            ErrorKind::Event => "event error",
            ErrorKind::Subscription => "event subscription error",
            ErrorKind::NoMoreLogs => "no more logs to pull",
            // ErrorKind::XmlParseError => "error parsing xml event",
        };

        write!(f, "{}", err_msg)
    }
}

impl Error {
    pub(crate) fn event(message: &str, error: io::Error) -> Self {
        Error {
            kind: ErrorKind::Event,
            message: format!("{} - ({})", message, error),
        }
    }

    pub(crate) fn subscription(message: &str, error: io::Error) -> Self {
        Error {
            kind: ErrorKind::Subscription,
            message: format!("{} - ({})", message, error),
        }
    }
}
