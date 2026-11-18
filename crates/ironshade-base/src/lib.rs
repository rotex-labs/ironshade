//! Source locations and diagnostics shared by the compiler and its tools.

use serde::{Deserialize, Serialize};

/// Identity of a source within a compilation session.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceId(pub u32);

/// A half-open range of UTF-8 byte offsets in one source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SpanFields")]
pub struct Span {
    source: SourceId,
    start: u32,
    end: u32,
}

#[derive(Deserialize)]
struct SpanFields {
    source: SourceId,
    start: u32,
    end: u32,
}

impl TryFrom<SpanFields> for Span {
    type Error = &'static str;

    fn try_from(fields: SpanFields) -> Result<Self, Self::Error> {
        Self::new(fields.source, fields.start, fields.end).ok_or("span end precedes start")
    }
}

impl Span {
    /// Returns a span if its end offset is at or after its start.
    pub fn new(source: SourceId, start: u32, end: u32) -> Option<Self> {
        (start <= end).then_some(Self { source, start, end })
    }

    /// The source containing this span.
    pub fn source(self) -> SourceId {
        self.source
    }

    /// The byte offsets covered by this span.
    pub fn range(self) -> std::ops::Range<u32> {
        self.start..self.end
    }
}

/// Diagnostic severity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Compilation could not proceed.
    Error,
    /// Compilation can proceed with a qualification.
    Warning,
}

/// A structured compiler message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable, machine-readable identifier.
    pub code: String,
    /// Importance of the message.
    pub severity: Severity,
    /// Explanation for the user.
    pub message: String,
    /// Source location, when available.
    pub span: Option<Span>,
}

impl Diagnostic {
    /// Constructs an error diagnostic.
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }
}

/// Diagnostics reported by a failed compiler operation.
pub type Diagnostics = Vec<Diagnostic>;

/// Programmable stage in the initial graphics and compute targets.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShaderStage {
    /// Vertex shader.
    Vertex,
    /// Fragment shader.
    Fragment,
    /// Compute shader.
    Compute,
}

impl ShaderStage {
    /// Stage spelling accepted by the oracle CLI.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Fragment => "fragment",
            Self::Compute => "compute",
        }
    }
}
