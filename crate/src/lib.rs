//! A two-way codec between ESTree and oxc's typed AST.
//!
//! The codec works across a string boundary: serialize a typed
//! [`oxc`](https://docs.rs/oxc) `Program` into an ESTree JSON string (and read
//! a string back into a typed `Program`), while the JS side parses that string
//! into an ESTree AST (and serializes an ESTree AST into a string).
//!
//! ## Usage
//!
//! **Rust → JS**: serialize a typed AST into an ESTree JSON string:
//!
//! ```rust,ignore
//! use oxc_estree_codec::{ProgramToJsonOptions, program_to_json};
//!
//! let json: String = program_to_json(&program, ProgramToJsonOptions::new());
//! ```
//!
//! ```ts
//! import type { Program } from "estree"; // npm i -D @types/estree
//!
//! const ast: Program = JSON.parse(json); // JSON string → ESTree AST
//! ```
//!
//! **JS → Rust**: read an ESTree JSON string into a typed AST:
//!
//! ```ts
//! const json: string = JSON.stringify(ast); // ESTree AST → JSON string
//! ```
//!
//! ```rust,ignore
//! use oxc::allocator::Allocator;
//! use oxc::ast::ast::{Program, SourceType};
//! use oxc_estree_codec::{JsonToProgramOptions, json_to_program};
//!
//! let json: &str = r#"{"type":"Program","body":[...],"sourceType":"module"}"#;
//!
//! let allocator: Allocator = Allocator::default();
//!
//! let source_text: &str = "const x = 1;";
//!
//! let program: Program<'_> = json_to_program(
//!     json,
//!     JsonToProgramOptions {
//!         allocator: &allocator,
//!         source_type: SourceType::from_path("index.js").unwrap(),
//!         source_text,
//!     },
//! )?;
//! ```

mod _types;
mod errors;
mod reader;

use oxc::ast::ast::Program;

use crate::reader::engine::program::ProgramReader;

pub use crate::_types::options::{JsonToProgramOptions, ProgramToJsonOptions};
pub use crate::errors::read::ReadError;

/// Serialize a typed oxc `Program` into an ESTree JSON string.
pub fn program_to_json(
    program: &Program<'_>,
    options: ProgramToJsonOptions,
) -> String {
    program.to_estree_json(options.include_ts_fields, options.ranges)
}

/// Read an ESTree JSON string into a typed oxc `Program`.
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
