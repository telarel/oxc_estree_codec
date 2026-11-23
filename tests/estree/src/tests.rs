use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use oxc::allocator::Allocator;
use oxc::ast::ast::{Program, TemplateElement};
use oxc::ast_visit::Visit;
use oxc::span::{SourceType, Span};
use oxc_estree_codec::{
    JsonToProgramOptions, ProgramToJsonOptions, SpanEncoding, json_to_program,
    program_to_json,
};
use test_helpers::corpus::collect_files;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
const CASES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/cases");

/// Hint shown when the generated fixtures are missing or incomplete.
const GENERATE_HINT: &str =
    "run `just estree-fixtures` to generate the fixtures";

/// A generated fixture and the source text it was parsed from.
struct Fixture {
    parser: String,
    source_name: String,
    json: String,
}

fn fixtures() -> Vec<Fixture> {
    let fixtures_dir: &Path = Path::new(FIXTURES_DIR);
    let manifest_path: PathBuf = fixtures_dir.join("manifest.json");

    // The generator writes the manifest last, so its presence marks a complete run.
    assert!(
        manifest_path.is_file(),
        "missing {}; {GENERATE_HINT}",
        manifest_path.display()
    );

    collect_files(fixtures_dir, "json")
        .into_iter()
        .filter(|path| path.file_name() != Some(OsStr::new("manifest.json")))
        .map(|path| {
            let relative: &Path =
                path.strip_prefix(fixtures_dir).unwrap_or_else(|error| {
                    panic!(
                        "fixture must be under {}: {error}",
                        fixtures_dir.display()
                    )
                });
            let parser: String = relative
                .components()
                .next()
                .unwrap_or_else(|| {
                    panic!(
                        "fixture must be inside a parser directory: {}",
                        path.display()
                    )
                })
                .as_os_str()
                .to_string_lossy()
                .into_owned();
            let file_name: &str = relative
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_else(|| {
                    panic!(
                        "fixture name must be valid UTF-8: {}",
                        path.display()
                    )
                });
            let source_name: &str = file_name
                .strip_suffix(".json")
                .expect("fixture file name must end with `.json`");
            let json: String =
                fs::read_to_string(&path).unwrap_or_else(|error| {
                    panic!("cannot read fixture {}: {error}", path.display())
                });

            Fixture { parser, source_name: source_name.to_owned(), json }
        })
        .collect()
}

/// Asserts every visited span is a UTF-8 byte range inside the source text.
struct SpanChecker<'s> {
    source: &'s str,
    /// Fixture label used in assertion messages.
    label: &'s str,
}

impl<'a> Visit<'a> for SpanChecker<'_> {
    fn visit_span(
        &mut self,
        span: &Span,
    ) {
        assert!(
            span.start <= span.end,
            "{}: span start {} exceeds end {}",
            self.label,
            span.start,
            span.end
        );
        assert!(
            (span.end as usize) <= self.source.len(),
            "{}: span end {} exceeds source length {}",
            self.label,
            span.end,
            self.source.len()
        );
        assert!(
            self.source.is_char_boundary(span.start as usize),
            "{}: span start {} is not a character boundary",
            self.label,
            span.start
        );
        assert!(
            self.source.is_char_boundary(span.end as usize),
            "{}: span end {} is not a character boundary",
            self.label,
            span.end
        );
    }
}

/// A decoded template element span, its raw value, and whether it is the tail.
struct TemplateElementSpan {
    span: Span,
    raw: String,
    tail: bool,
}

/// Collects template elements so their spans can be checked against `value.raw`.
struct TemplateChecker<'s> {
    source: &'s str,
    /// Fixture label used in assertion messages.
    label: &'s str,
    elements: Vec<TemplateElementSpan>,
}

impl TemplateChecker<'_> {
    fn check(&self) {
        for element in &self.elements {
            let start: usize = element.span.start as usize;
            let end: usize = element.span.end as usize;
            let slice: &str = self.source.get(start..end).unwrap_or_else(|| {
                panic!(
                    "{}: template element span {start}..{end} is outside the source",
                    self.label
                )
            });

            assert_eq!(
                slice, element.raw,
                "{}: template element span {start}..{end} must cover value.raw",
                self.label
            );
            assert_eq!(
                element.span.size(),
                element.raw.len() as u32,
                "{}: template element span {start}..{end} must match the raw byte length (tail: {})",
                self.label,
                element.tail
            );
        }
    }
}

impl<'a> Visit<'a> for TemplateChecker<'_> {
    fn visit_template_element(
        &mut self,
        element: &TemplateElement<'a>,
    ) {
        self.elements.push(TemplateElementSpan {
            span: element.span,
            raw: element.value.raw.as_str().to_owned(),
            tail: element.tail,
        });
    }
}

#[test]
fn test_third_party_estree_fixtures_decode() {
    let fixtures: Vec<Fixture> = fixtures();

    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {FIXTURES_DIR}; {GENERATE_HINT}"
    );

    let mut template_elements: usize = 0;

    for fixture in &fixtures {
        let label: String =
            format!("{}/{}", fixture.parser, fixture.source_name);

        let source_path: PathBuf =
            Path::new(CASES_DIR).join(&fixture.source_name);

        let source_text: String = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| {
                panic!(
                    "{label}: cannot read source {}: {error}",
                    source_path.display()
                )
            });

        let source_type: SourceType =
            SourceType::from_path(&fixture.source_name)
                .unwrap_or_else(|error| panic!("{label}: {error}"));

        let allocator: Allocator = Allocator::default();

        let program: Program<'_> = json_to_program(
            &fixture.json,
            JsonToProgramOptions {
                allocator: &allocator,
                source_type,
                source_text: &source_text,
                span_encoding: SpanEncoding::Utf16Units,
            },
        )
        .unwrap_or_else(|error| panic!("{label}: {error}"));

        let mut span_checker: SpanChecker<'_> =
            SpanChecker { source: &source_text, label: &label };

        span_checker.visit_program(&program);

        let mut template_checker: TemplateChecker<'_> = TemplateChecker {
            source: &source_text,
            label: &label,
            elements: Vec::new(),
        };

        template_checker.visit_program(&program);

        template_checker.check();

        template_elements += template_checker.elements.len();

        let json1: String =
            program_to_json(&program, ProgramToJsonOptions::new());

        let program2: Program<'_> = json_to_program(
            &json1,
            JsonToProgramOptions {
                allocator: &allocator,
                source_type,
                source_text: &source_text,
                span_encoding: SpanEncoding::Utf8Bytes,
            },
        )
        .unwrap_or_else(|error| {
            panic!("{label}: re-decode after encode: {error}")
        });

        let json2: String =
            program_to_json(&program2, ProgramToJsonOptions::new());

        assert_eq!(
            json1, json2,
            "{label}: encode/decode/encode must be byte-identical"
        );
    }

    assert!(
        template_elements > 0,
        "the fixtures must contain template elements"
    );

    println!("third-party fixtures: {}", fixtures.len());
}
