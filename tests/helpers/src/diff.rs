use oxc::allocator::Allocator;
use oxc::span::SourceType;

use oxc_estree_codec::{
    JsonToProgramOptions, ProgramToJsonOptions, json_to_program,
    program_to_json,
};

pub enum RoundtripError {
    ParseFailed,
    Roundtrip(String),
}

pub fn roundtrip_bytes_with(
    file: &str,
    code: &str,
    source_type: SourceType,
) -> Result<String, RoundtripError> {
    let allocator: Allocator = Allocator::default();

    let parser_return: oxc::parser::ParserReturn<'_> =
        oxc::parser::Parser::new(&allocator, code, source_type)
            .with_options(oxc::parser::ParseOptions {
                parse_regular_expression: true,
                ..oxc::parser::ParseOptions::default()
            })
            .parse();

    if !parser_return.diagnostics.is_empty() {
        return Err(RoundtripError::ParseFailed);
    }

    let semantic_return: oxc::semantic::SemanticBuilderReturn<'_> =
        oxc::semantic::SemanticBuilder::new()
            .with_check_syntax_error(true)
            .build(&parser_return.program);

    if semantic_return.diagnostics.has_errors() {
        return Err(RoundtripError::ParseFailed);
    }

    let before: String = program_to_json(
        &parser_return.program,
        ProgramToJsonOptions::default(),
    );

    let program: oxc::ast::ast::Program<'_> = json_to_program(
        &before,
        JsonToProgramOptions {
            allocator: &allocator,
            source_type: parser_return.program.source_type,
            source_text: parser_return.program.source_text,
        },
    )
    .map_err(|error| RoundtripError::Roundtrip(format!("{file}: {error}")))?;

    let after: String =
        program_to_json(&program, ProgramToJsonOptions::default());

    if before != after {
        return Err(RoundtripError::Roundtrip(format!(
            "roundtrip must be byte-stable: {file}"
        )));
    }

    Ok(after)
}

pub fn roundtrip_bytes(
    file: &str,
    code: &str,
) -> Result<String, RoundtripError> {
    roundtrip_bytes_with(
        file,
        code,
        SourceType::from_path(file).unwrap_or_default(),
    )
}
