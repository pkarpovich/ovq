use ignore::WalkBuilder;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

pub fn collect_markdown_files(vault_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(vault_path)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .add_custom_ignore_filename(".obsidianignore")
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
            files.push(path.to_path_buf());
        }
    }

    files
}

pub fn read_paths_from_stdin() -> Vec<PathBuf> {
    let stdin = io::stdin();
    stdin
        .lock()
        .lines()
        .flatten()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect()
}
