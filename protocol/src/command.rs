//! The injector → agent_loader command set and its wire encoding.

use std::path::PathBuf;

use thiserror::Error;

/// A single command sent from the injector to the agent loader.
///
/// The wire form is one UTF-8 line: a lowercase verb, optionally followed by
/// a single space and one argument that runs to the end of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Load — or hot-reload — the client library at the given absolute path.
    Reload(PathBuf),
}

/// Failure to parse a command off the wire.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    /// The line carried no verb.
    #[error("empty command line")]
    Empty,
    /// The verb is not one this protocol version understands.
    #[error("unknown command verb: {0:?}")]
    UnknownVerb(String),
    /// A verb that requires an argument was sent without one.
    #[error("command {verb:?} is missing its argument")]
    MissingArgument { verb: &'static str },
}

impl Command {
    /// Serializes the command into its single-line wire form (no newline).
    pub fn encode(&self) -> String {
        match self {
            Command::Reload(path) => format!("reload {}", path.display()),
        }
    }

    /// Parses one wire line back into a [`Command`].
    ///
    /// The argument runs to the end of the line, so paths containing spaces
    /// survive the round trip.
    pub fn decode(line: &str) -> Result<Self, ProtocolError> {
        let line = line.trim();
        if line.is_empty() {
            return Err(ProtocolError::Empty);
        }
        let (verb, arg) = match line.split_once(' ') {
            Some((verb, arg)) => (verb, arg.trim()),
            None => (line, ""),
        };
        match verb {
            "reload" => {
                if arg.is_empty() {
                    Err(ProtocolError::MissingArgument { verb: "reload" })
                } else {
                    Ok(Command::Reload(PathBuf::from(arg)))
                }
            }
            other => Err(ProtocolError::UnknownVerb(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn reload_round_trips() {
        let command = Command::Reload(PathBuf::from("/tmp/libclient.so"));
        let wire = command.encode();
        assert_eq!(wire, "reload /tmp/libclient.so");
        assert_eq!(Command::decode(&wire), Ok(command));
    }

    #[test]
    fn reload_path_with_spaces_survives_the_round_trip() {
        let command = Command::Reload(PathBuf::from("/home/My Games/libclient.so"));
        assert_eq!(Command::decode(&command.encode()), Ok(command));
    }

    #[test]
    fn decode_ignores_surrounding_whitespace() {
        assert_eq!(
            Command::decode("  reload /a/b.so\n"),
            Ok(Command::Reload(PathBuf::from("/a/b.so"))),
        );
    }

    #[test]
    fn an_empty_line_is_rejected() {
        assert_eq!(Command::decode("   "), Err(ProtocolError::Empty));
    }

    #[test]
    fn reload_without_an_argument_is_rejected() {
        assert_eq!(
            Command::decode("reload"),
            Err(ProtocolError::MissingArgument { verb: "reload" }),
        );
    }

    #[test]
    fn an_unknown_verb_is_rejected() {
        assert_eq!(
            Command::decode("frobnicate x"),
            Err(ProtocolError::UnknownVerb("frobnicate".to_string())),
        );
    }
}
