use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    SourceBytes,
    SyntaxNodes,
    NestingDepth,
}

impl fmt::Display for LimitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::SourceBytes => "source bytes",
            Self::SyntaxNodes => "syntax nodes",
            Self::NestingDepth => "nesting depth",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum Error {
    #[error("parse error at byte {offset}: {message}")]
    Parse {
        offset: usize,
        message: &'static str,
    },
    #[error("{0}")]
    UnknownVariable(&'static str),
    #[error("{0}")]
    UnknownFunction(&'static str),
    #[error("type error: {0}")]
    Type(&'static str),
    #[error("arithmetic error: {0}")]
    Arithmetic(&'static str),
    #[error("{kind} limit exceeded (limit {limit})")]
    Limit { kind: LimitKind, limit: usize },
}

impl Error {
    pub(crate) const fn parse(offset: usize, message: &'static str) -> Self {
        Self::Parse { offset, message }
    }

    pub(crate) fn limit(kind: LimitKind, limit: usize) -> Self {
        Self::Limit { kind, limit }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
