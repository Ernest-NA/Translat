use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceError {
    context: String,
    details: Option<String>,
}

impl PersistenceError {
    pub fn new(context: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            details: None,
        }
    }

    pub fn with_details(context: impl Into<String>, details: impl Display) -> Self {
        Self {
            context: context.into(),
            details: Some(details.to_string()),
        }
    }
}

impl Display for PersistenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(details) = &self.details {
            write!(formatter, "{} ({details})", self.context)
        } else {
            formatter.write_str(&self.context)
        }
    }
}

impl Error for PersistenceError {}
