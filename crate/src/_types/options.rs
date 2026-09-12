use oxc::allocator::Allocator;
use oxc::ast::ast::SourceType;

#[derive(Debug, Clone, Copy)]
pub struct ProgramToJsonOptions {
    pub include_ts_fields: bool,
    pub ranges: bool,
}

impl Default for ProgramToJsonOptions {
    fn default() -> Self {
        Self { include_ts_fields: true, ranges: false }
    }
}

#[derive(Clone, Copy)]
pub struct JsonToProgramOptions<'a> {
    pub allocator: &'a Allocator,
    pub source_type: SourceType,
    pub source_text: &'a str,
}
