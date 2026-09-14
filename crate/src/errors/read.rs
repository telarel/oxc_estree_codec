use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    message: String,
}

impl ReadError {
    pub fn from_message(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl Display for ReadError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ReadError {}
