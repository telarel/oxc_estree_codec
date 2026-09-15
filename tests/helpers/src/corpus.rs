use std::fs;
use std::path::{Path, PathBuf};

use workspace_root::get_workspace_root;

pub fn corpus_root(name: &str) -> PathBuf {
    get_workspace_root().join(name)
}

pub fn collect_files(
    dir: &Path,
    extension: &str,
) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };

        for entry in entries.flatten() {
            let path: PathBuf = entry.path();

            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == extension) {
                files.push(path);
            }
        }
    }

    files.sort();

    files
}

// Each corpus test runs on a big-stack thread: the parser and reader recurse
// per AST depth, and the debug test binary needs more than the default 8 MB
// test-thread stack on deep test262 files.
pub fn run_on_big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(f)
        .expect("spawn corpus thread")
        .join()
        .unwrap_or_else(|panic_payload| {
            std::panic::resume_unwind(panic_payload)
        });
}
