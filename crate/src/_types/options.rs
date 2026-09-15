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

#[derive(Clone, Copy)]
pub struct JsonToProgramOptions<'a> {
    /// Arena allocator the resulting AST nodes are allocated in.
    pub allocator: &'a Allocator,
    /// Source type of the program; resolves unambiguous values from the JSON.
    pub source_type: SourceType,
    /// Original source text, used to build string atoms from the program.
    pub source_text: &'a str,
}
