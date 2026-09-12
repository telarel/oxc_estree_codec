use oxc::allocator::Allocator;
use oxc::ast::ast::SourceType;

#[derive(Clone, Copy)]
pub struct FromJsonOptions<'a> {
    pub allocator: &'a Allocator,
    pub source_type: SourceType,
    pub source_text: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ToJsonOptions {
    pub include_ts_fields: bool,
    pub ranges: bool,
}

impl Default for ToJsonOptions {
    fn default() -> Self {
        Self { include_ts_fields: true, ranges: false }
    }
}
