use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadError {
    JsonInvalid {
        message: String,
    },
    NodeUnsupported {
        ty: String,
        path: String,
    },
    FieldMissing {
        field: &'static str,
        ty: String,
        path: String,
    },
    FieldInvalid {
        field: &'static str,
        expected: &'static str,
        ty: String,
        path: String,
    },
    OperatorUnsupported {
        kind: &'static str,
        operator: String,
        path: String,
    },
    ValueUnsupported {
        kind: &'static str,
        value: String,
        path: String,
    },
    ImportPhaseUnsupported {
        phase: String,
        path: String,
    },
}

impl ReadError {
    pub fn from_message(message: impl Into<String>) -> Self {
        Self::JsonInvalid { message: message.into() }
    }
}

impl Display for ReadError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            | Self::JsonInvalid { message } => f.write_str(message),
            | Self::NodeUnsupported { ty, path } => {
                write!(f, "unsupported ESTree node type `{ty}` at {path}")
            },
            | Self::FieldMissing { field, ty, path } => {
                write!(f, "missing field `{field}` on `{ty}` node at {path}")
            },
            | Self::FieldInvalid { field, expected, ty, path } => {
                write!(
                    f,
                    "invalid field `{field}` on `{ty}` node at {path}: expected {expected}"
                )
            },
            | Self::OperatorUnsupported { kind, operator, path } => {
                write!(f, "unsupported {kind} `{operator}` at {path}")
            },
            | Self::ValueUnsupported { kind, value, path } => {
                write!(f, "unsupported {kind} `{value}` at {path}")
            },
            | Self::ImportPhaseUnsupported { phase, path } => {
                write!(f, "unsupported import phase `{phase}` at {path}")
            },
        }
    }
}

impl std::error::Error for ReadError {}
