mod _types;
mod errors;
mod reader;

use oxc::ast::ast::Program;

use crate::reader::engine::program::ProgramReader;

pub use crate::_types::options::{JsonToProgramOptions, ProgramToJsonOptions};
pub use crate::errors::read::ReadError;

pub fn program_to_json(
    program: &Program<'_>,
    options: ProgramToJsonOptions,
) -> String {
    program.to_estree_json(options.include_ts_fields, options.ranges)
}

pub fn json_to_program<'j, 'a>(
    json: &'j str,
    options: JsonToProgramOptions<'a>,
) -> Result<Program<'a>, ReadError> {
    let value: crate::reader::json::Value =
        crate::reader::json::parse(json).map_err(ReadError::from_message)?;

    let reader: ProgramReader<'a> = ProgramReader::new(options.allocator);

    reader.read(&value, options.source_type, options.source_text)
}

#[doc(hidden)]
pub mod __internal {
    use oxc::allocator::Allocator;
    use oxc::ast::ast::Program;

    use crate::_types::options::{JsonToProgramOptions, ProgramToJsonOptions};
    use crate::errors::read::ReadError;
    use crate::{json_to_program, program_to_json};

    pub use crate::reader::engine::program::ProgramReader;

    pub fn roundtrip<'a>(
        allocator: &'a Allocator,
        program: &Program<'a>,
    ) -> Result<Program<'a>, ReadError> {
        let json: String =
            program_to_json(program, ProgramToJsonOptions::default());

        json_to_program(
            &json,
            JsonToProgramOptions {
                allocator,
                source_type: program.source_type,
                source_text: program.source_text,
            },
        )
    }
}
