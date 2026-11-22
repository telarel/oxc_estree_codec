use oxc::allocator::Allocator;
use oxc::ast::ast::SourceType;

#[derive(Debug, Clone, Copy)]
pub struct ProgramToJsonOptions {
    /// Whether to include TypeScript-specific fields in the output.
    ///
    /// Defaults to `true`.
    pub include_ts_fields: bool,
    /// Whether to include legacy `range` arrays on nodes.
    ///
    /// Defaults to `false`.
    pub ranges: bool,
}

impl ProgramToJsonOptions {
    pub fn new() -> Self {
        Self { include_ts_fields: true, ranges: false }
    }
}

impl Default for ProgramToJsonOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// How `start`/`end` and `range` offsets in the ESTree JSON are encoded.
///
/// By default, it is `Utf8Bytes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanEncoding {
    /// UTF-8 byte offsets.
    ///
    /// This is what oxc's ESTree serializer emits.
    Utf8Bytes,
    /// UTF-16 code unit offsets.
    ///
    /// This is what JavaScript parsers emit.
    Utf16Units,
}

impl SpanEncoding {
    /// Create a span encoding.
    pub fn new() -> Self {
        Self::Utf8Bytes
    }
}

impl Default for SpanEncoding {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub struct JsonToProgramOptions<'a> {
    /// Arena allocator the resulting AST nodes are allocated in.
    pub allocator: &'a Allocator,
    /// Source type of the program; resolves unambiguous values from the JSON.
    pub source_type: SourceType,
    /// Original source text, used to build string atoms from the program.
    pub source_text: &'a str,
    /// How `start`/`end` and `range` offsets are encoded.
    pub span_encoding: SpanEncoding,
}
