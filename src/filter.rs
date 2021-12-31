use walkdir::{DirEntry, WalkDir};

pub fn is_markdown(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(".md"))
        .unwrap_or(false)
}

pub fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with('.') && !s.starts_with('_'))
        .unwrap_or(false)
}

pub fn is_directory(entry: &DirEntry) -> bool {
    entry.metadata().map(|s| s.is_dir()).unwrap_or(false)
}

pub fn is_image(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|f| {
            f.ends_with(".jpg")
                || f.ends_with(".JPG")
                || f.ends_with(".png")
                || f.ends_with(".PNG")
                || f.ends_with(".gif")
                || f.ends_with(".GIF")
        })
        .unwrap_or(false)
}

pub fn contains_markdown_file(entry: &DirEntry) -> bool {
    WalkDir::new(entry.path())
        .into_iter()
        .any(|e| e.is_ok() && is_markdown(e.as_ref().unwrap()))
}

pub fn contains_markdown_subdir(entry: &DirEntry) -> bool {
    WalkDir::new(entry.path())
        .min_depth(1)
        .into_iter()
        .any(|e| {
            e.is_ok()
                && is_directory(e.as_ref().unwrap())
                && contains_markdown_file(e.as_ref().unwrap())
        })
}
