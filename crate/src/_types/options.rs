use oxc::allocator::Allocator;
use oxc::ast::ast::SourceType;

/// Options for serializing an oxc `Program` to an ESTree JSON payload.
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
    /// Create options with TypeScript fields enabled and legacy `range` arrays disabled.
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

/// Options for reading an ESTree JSON payload into an oxc `Program`.
#[derive(Clone, Copy)]
pub struct JsonToProgramOptions<'a> {
    /// Arena allocator the resulting AST nodes are allocated in.
    pub allocator: &'a Allocator,
    /// Source type of the program.
    ///
    /// When this value is unambiguous, a valid JSON `sourceType` refines it.
    /// An explicit module, script, or CommonJS value wins.
    pub source_type: SourceType,
    /// Original source text.
    ///
    /// The reader stores it on the resulting `Program`, uses it to map
    /// UTF-16 offsets, and uses it to validate template element spans.
    /// UTF-16 offsets must be within this text and on a UTF-8 character
    /// boundary. Template element spans must
    /// match `value.raw` after CRLF/CR normalization, under either the
    /// content-only or delimiter-inclusive convention. For template element
    /// validation, an empty source text uses the declared span lengths
    /// instead.
    pub source_text: &'a str,
    /// How `start`/`end` and `range` offsets are encoded.
    ///
    /// A node uses `start` and `end` when both are present; `range` is the
    /// fallback only when both are absent. A partial `start`/`end` pair is an
    /// error. Missing span information is accepted and produces a `0/0` span.
    /// Invalid span values return [`crate::ReadError`].
    pub span_encoding: SpanEncoding,
}
