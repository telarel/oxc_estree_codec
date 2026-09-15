#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use test_helpers::corpus::collect_files;
    use test_helpers::diff::{RoundtripError, roundtrip_bytes};

    const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data");

    #[test]
    fn test_corpus_checked_in_roundtrip() {
        let ts_dir: PathBuf = Path::new(FIXTURES_DIR).join("ts");

        let tsx_dir: PathBuf = Path::new(FIXTURES_DIR).join("tsx");

        let mut cases: Vec<PathBuf> = Vec::new();

        cases.extend(collect_files(&ts_dir, "ts"));

        cases.extend(collect_files(&tsx_dir, "tsx"));

        assert!(
            cases.len() >= 16,
            "checked-in corpus must be present: {cases:?}"
        );

        let mut failures: Vec<String> = Vec::new();

        for path in &cases {
            let code: String =
                fs::read_to_string(path).expect("fixture must be readable");

            match roundtrip_bytes(path.to_string_lossy().as_ref(), &code) {
                | Ok(_) => {},
                | Err(RoundtripError::ParseFailed) => {
                    failures.push(format!("fixture does not parse: {path:?}"));
                },
                | Err(RoundtripError::Roundtrip(message)) => {
                    failures.push(message);
                },
            }
        }

        assert!(
            failures.is_empty(),
            "{} of {} corpus files failed:\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}
