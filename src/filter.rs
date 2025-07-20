use std::{path::Path, time::SystemTime};
use walkdir::{DirEntry, WalkDir};

pub fn is_markdown(entry: &DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(|s| {
        Path::new(s)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    })
}

pub fn is_modified_markdown(entry: &DirEntry, last_build: SystemTime) -> bool {
    let is_markdown = is_markdown(entry);
    if is_markdown
        && let Ok(metadata) = entry.metadata()
        && let Ok(modified) = metadata.modified()
    {
        return modified > last_build;
    }
    is_markdown
}

pub fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| !s.starts_with('.') && !s.starts_with('_'))
}

pub fn is_directory(entry: &DirEntry) -> bool {
    entry.metadata().map(|s| s.is_dir()).unwrap_or(false)
}

pub fn is_image(entry: &DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(|s| {
        Path::new(s).extension().is_some_and(|ext| {
            ext.eq_ignore_ascii_case("jpg")
                || ext.eq_ignore_ascii_case("png")
                || ext.eq_ignore_ascii_case("gif")
        })
    })
}

pub fn contains_markdown_file(entry: &DirEntry) -> bool {
    WalkDir::new(entry.path())
        .into_iter()
        .any(|e| e.is_ok() && is_markdown(e.as_ref().unwrap()))
}

pub fn contains_markdown_in_dir(entry: &DirEntry) -> bool {
    WalkDir::new(entry.path())
        .max_depth(1)
        .into_iter()
        .any(|e| e.is_ok_and(|e| is_markdown(&e)))
}

pub fn contains_markdown_subdir(entry: &DirEntry) -> bool {
    WalkDir::new(entry.path())
        .min_depth(1)
        .into_iter()
        .any(|e| e.is_ok_and(|e| is_directory(&e) && contains_markdown_file(&e)))
}
