use crate::config::Config;
use crate::filter::{
    contains_markdown_file, contains_markdown_subdir, is_directory, is_image, is_modified_markdown,
    is_not_hidden,
};
use crate::template;
use anyhow::Result;
use lazy_static::lazy_static;
use pulldown_cmark::{html, Options, Parser};
use rayon::prelude::*;
use regex::Regex;
use std::fmt::{self, Debug};
use std::fs;
use std::fs::DirBuilder;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::vec::Vec;
use walkdir::{DirEntry, WalkDir};

static BUILD_TIMESTAMP_FILE: &str = "last_build.json";

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuCmd {
    OpenLevel,
    CloseLevel,
    CloseOpenLevel,
    None,
}

impl fmt::Display for MenuCmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

pub fn build(path: &Path, conf: &Config, clean: bool) -> Result<()> {
    log::info!(
        "[{}] Building project `{}`",
        time::OffsetDateTime::now_local()
            .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_else(|_| "??".to_string()),
        conf.title.as_deref().unwrap_or("unknown"),
    );
    let output_dir = PathBuf::from(conf.output_dir.as_ref().unwrap());
    if clean {
        fs::remove_dir_all(&output_dir)?;
    }
    let nav_items = prepare_site_structure(path, output_dir.as_path(), conf);
    let prev_build_timestamp = get_and_set_build_timestamp(output_dir.as_path());
    let entries: Vec<_> = WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_entry(is_not_hidden)
        .filter(|e| e.is_ok() && is_modified_markdown(e.as_ref().unwrap(), prev_build_timestamp))
        .collect();
    entries.par_iter().for_each(|e| {
        let src = e.as_ref().unwrap();
        let mut target_dir = output_dir.clone();
        if let Some(parent_path) = src.path().parent() {
            target_dir.extend(parent_path.components().skip(path.components().count()));
        }
        build_page(nav_items.clone(), src, target_dir.as_path(), conf);
    });
    copy_dirs(path, output_dir.as_path(), conf);
    Ok(())
}

fn get_and_set_build_timestamp(outdir: &Path) -> SystemTime {
    let now = SystemTime::now();
    let build_timestamp_file = outdir.join(BUILD_TIMESTAMP_FILE);
    let mut last_build = SystemTime::UNIX_EPOCH;
    if let Ok(f) = std::fs::File::open(&build_timestamp_file) {
        if let Ok(build_tstmp) = serde_json::from_reader(f) {
            log::info!(
                "Duration since last build {:#?}",
                now.duration_since(build_tstmp).unwrap_or_default()
            );
            last_build = build_tstmp;
        }
    }
    let f = std::fs::File::create(&build_timestamp_file)
        .expect("Unable to create {build_timestamp_file:?}");
    serde_json::to_writer(f, &now).expect("Unable to write data to {build_timestamp_file:?}");
    last_build
}

fn build_page(
    nav_items: Vec<liquid::model::Value>,
    entry: &DirEntry,
    target_dir: &Path,
    conf: &Config,
) {
    let page_name = target_dir
        .file_name()
        .map_or("None", |name| name.to_str().unwrap_or("None"));
    let page_content = convert_markdown_to_html(entry.path());
    let html = if entry.file_name() == "gallery.md" {
        let images = prepare_gallery(entry, target_dir, conf);
        apply_gallery_template(
            &page_content,
            nav_items,
            entry.depth(),
            conf,
            page_name,
            images,
        )
    } else {
        apply_page_template(&page_content, nav_items, entry.depth(), conf, page_name)
    };
    write_html_file(&html, target_dir, entry);
    copy_images(entry.path().parent().unwrap(), target_dir);
}

fn is_file_modified(src: &Path, trg: &Path) -> bool {
    if let Ok(src_meta) = src.metadata() {
        if let Ok(trg_meta) = trg.metadata() {
            if let Ok(src_tstmp) = src_meta.modified() {
                if let Ok(trg_tstmp) = trg_meta.modified() {
                    return src_tstmp != trg_tstmp;
                }
            }
        }
    }
    true
}

fn copy_dirs(path: &Path, target_path: &Path, conf: &Config) {
    if conf.copy_dirs.is_none() {
        return;
    }
    for copy_dir in conf.copy_dirs.as_ref().unwrap() {
        let walker = WalkDir::new(path.join(copy_dir).as_path())
            .min_depth(1)
            .into_iter();
        for entry in walker.filter(|e| e.is_ok() && !is_directory(e.as_ref().unwrap())) {
            let entry = entry.unwrap();
            let mut target_file = PathBuf::from(target_path);
            target_file.extend(entry.path().components().skip(path.components().count()));
            if let Err(ref err) = DirBuilder::new()
                .recursive(true)
                .create(target_file.parent().expect("Missing parent folder!"))
            {
                println!("{err}");
                log::error!("{err}");
            }
            if !target_file.exists() || is_file_modified(entry.path(), &target_file) {
                fs::copy(entry.path(), target_file)
                    .unwrap_or_else(|_| panic!("error during copy of {copy_dir:?}"));
            }
        }
    }
}

fn copy_images(source: &Path, target: &Path) {
    let walker = WalkDir::new(source)
        .min_depth(1)
        .max_depth(1)
        .follow_links(true)
        .into_iter();
    for entry in walker
        .filter(|e| e.is_ok() && is_image(e.as_ref().unwrap()))
        .flatten()
    {
        let mut target_file = target.to_path_buf();
        target_file.push(entry.path().file_name().unwrap());
        if !target_file.exists() || is_file_modified(entry.path(), &target_file) {
            fs::copy(entry.path(), target_file.as_path())
                .unwrap_or_else(|_| panic!("Error during copy of {:?}", entry.path().display()));
        }
    }
}

fn remove_number_prefix<'a>(name: &'a str, conf: &Config) -> &'a str {
    lazy_static! {
        static ref NUM_PREFIX: Regex = Regex::new("^[0-9]+_.+$").unwrap();
    }
    if (conf.remove_numbered_prefix.is_none() || conf.remove_numbered_prefix.unwrap_or(true))
        && NUM_PREFIX.is_match(name)
    {
        let mut splitter = name.splitn(2, '_');
        splitter.next();
        splitter.next().unwrap()
    } else {
        name
    }
}

fn prepare_site_structure(
    path: &Path,
    target_path: &Path,
    conf: &Config,
) -> Vec<liquid::model::Value> {
    let mut nav_entries = Vec::<liquid::model::Value>::new();
    let walker = WalkDir::new(path)
        .min_depth(1)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .into_iter();
    let mut prev_depth = 1;
    for entry in
        walker.filter_entry(|e| is_not_hidden(e) && is_directory(e) && contains_markdown_file(e))
    {
        let entry = entry.expect("Reading directory entry failed");
        let name = String::from(remove_number_prefix(
            entry
                .file_name()
                .to_str()
                .expect("Failed to read navigation entries"),
            conf,
        ));
        let mut url = PathBuf::new();
        url.extend(entry.path().components().skip(path.components().count()));
        let target_dir = target_path.join(url.as_path());
        if let Err(ref err) = DirBuilder::new().recursive(true).create(target_dir) {
            println!("{err}");
            log::error!("{err}");
        }
        url.push("index.html");
        let (menu_cmd, level_depth) =
            match (contains_markdown_subdir(&entry), prev_depth > entry.depth()) {
                (true, true) => (MenuCmd::CloseOpenLevel, prev_depth - entry.depth() - 1),
                (true, false) => (MenuCmd::OpenLevel, 0),
                (false, true) => (MenuCmd::CloseLevel, prev_depth - entry.depth() - 1),
                _ => (MenuCmd::None, 0),
            };
        let nav_entry = liquid::object!({
            "name": name,
            "url" : url.as_os_str().to_str().unwrap().to_owned(),
            "menu_cmd" : menu_cmd.to_string().clone(),
            "level_depth" : level_depth,
        });
        nav_entries.push(liquid::model::Value::Object(nav_entry));
        prev_depth = entry.depth();
    }
    if prev_depth > 1 {
        let nav_entry = liquid::object!({
            "name": String::new(),
            "url" : String::new(),
            "menu_cmd" : MenuCmd::CloseLevel.to_string(),
            "level_depth" : prev_depth - 1,
        });
        nav_entries.push(liquid::model::Value::Object(nav_entry));
    }
    nav_entries
}

fn prepare_gallery(
    source_entry: &DirEntry,
    target_path: &Path,
    conf: &Config,
) -> Vec<liquid::model::Value> {
    let gallery_settings = conf.gallery.as_ref().unwrap();
    let mut images = Vec::<liquid::model::Value>::new();
    let img_dir = gallery_settings.img_dir.as_ref().unwrap();
    let target_dir = target_path.join(img_dir.as_str());
    if let Err(ref err) = DirBuilder::new().recursive(true).create(&target_dir) {
        println!("{err}");
        log::error!("{err}");
    }
    let entries = WalkDir::new(
        source_entry
            .path()
            .parent()
            .unwrap()
            .join(img_dir.as_str())
            .as_path(),
    )
    .min_depth(1)
    .follow_links(true)
    .into_iter()
    .filter(|e| e.is_ok() && !is_directory(e.as_ref().unwrap()))
    .collect::<Vec<_>>();
    for entry in entries {
        let entry = entry.unwrap();
        let mut img = image::open(entry.path()).unwrap_or_else(|_| {
            panic!(
                "Resize of '{}' failed: The gallery folder should only contain images!",
                entry.path().display()
            )
        });

        let mut image_path = PathBuf::from(&target_dir);
        let mut rel_image_path = PathBuf::from(img_dir.as_str());
        image_path.push(entry.file_name());
        image_path.set_extension("png");
        rel_image_path.push(entry.file_name());
        rel_image_path.set_extension("png");
        if !image_path.exists() {
            let _ = &mut File::create(&image_path).unwrap();
            img = img.resize(
                gallery_settings.img_width,
                gallery_settings.img_height,
                image::imageops::FilterType::Nearest,
            );
            img.save_with_format(&image_path, image::ImageFormat::Png)
                .unwrap_or_else(|_| panic!("Saving image '{}' failed", image_path.display()));
        }

        let mut thumb_path = PathBuf::from(&target_dir);
        let mut rel_thumb_path = PathBuf::from(img_dir.as_str());
        let mut thumb_file_name = String::from(
            entry
                .path()
                .file_stem()
                .map(|s| s.to_str().unwrap())
                .unwrap(),
        );
        thumb_file_name.push_str("_thumb.png");
        thumb_path.push(thumb_file_name.clone());
        rel_thumb_path.push(thumb_file_name);
        if !thumb_path.exists() {
            let _ = &mut File::create(&thumb_path).unwrap();
            img = img.resize(
                gallery_settings.thumb_width,
                gallery_settings.thumb_height,
                image::imageops::FilterType::Nearest,
            );
            img.save_with_format(&thumb_path, image::ImageFormat::Png)
                .unwrap_or_else(|_| panic!("Saving thumb image '{}' failed", thumb_path.display()));
        }

        let image_entry = liquid::object!({
            "name"  : rel_image_path.to_str().unwrap().to_owned(),
            "thumb" : rel_thumb_path.to_str().unwrap().to_owned(),
        });
        images.push(liquid::model::Value::Object(image_entry));
    }
    images
}

fn apply_gallery_template(
    content: &str,
    nav_items: Vec<liquid::model::Value>,
    depth: usize,
    conf: &Config,
    page_name: &str,
    images: Vec<liquid::model::Value>,
) -> String {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .unwrap()
        .parse(template::load_gallery(conf).as_str())
        .expect("Gallery template could not be parsed!");
    let mut root_dir = String::new();
    for _ in 1..depth {
        root_dir.push_str("../");
    }
    let mut context = liquid::model::Object::new();
    context.insert("root_dir".into(), liquid::model::Value::scalar(root_dir));
    context.insert(
        "title".into(),
        liquid::model::Value::scalar(if conf.title.is_some() {
            conf.title.as_ref().unwrap().clone()
        } else {
            "None".to_string()
        }),
    );
    context.insert("nav_items".into(), liquid::model::Value::Array(nav_items));
    context.insert(
        "content".into(),
        liquid::model::Value::scalar(content.to_owned()),
    );
    context.insert("images".into(), liquid::model::Value::Array(images));
    context.insert(
        "page_name".into(),
        liquid::model::Value::scalar(page_name.to_owned()),
    );
    match template.render(&context) {
        Ok(output) => output,
        Err(error) => panic!("Could not render Page template: {error}"),
    }
}

fn apply_page_template(
    content: &str,
    nav_items: Vec<liquid::model::Value>,
    depth: usize,
    conf: &Config,
    page_name: &str,
) -> String {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .unwrap()
        .parse(template::load_page(conf).as_str())
        .expect("Page template could not be parsed!");
    let mut root_dir = String::from("./");
    for _ in 1..depth {
        root_dir.push_str("../");
    }
    let context = liquid::object!({
       "root_dir" : root_dir,
       "title" : if conf.title.is_some() {
            conf.title.as_ref().unwrap().clone()
        } else {
            "None".to_string()
        },
        "nav_items" : liquid::model::Value::Array(nav_items),
        "content" : content.to_owned(),
        "page_name" : page_name.to_owned()
    });
    match template.render(&context) {
        Ok(output) => output,
        Err(error) => panic!("Could not render Page template: {error}"),
    }
}

fn convert_markdown_to_html(entry: &Path) -> String {
    let mut markdown = String::new();
    let mut html_output = String::new();
    match File::open(entry).and_then(|mut f| f.read_to_string(&mut markdown)) {
        Err(error) => panic!("failed to open {}: {}", entry.display(), error),
        Ok(_) => html::push_html(
            &mut html_output,
            Parser::new_ext(markdown.as_str(), Options::empty()),
        ),
    }
    html_output
}

fn write_html_file(html: &str, target_dir: &Path, entry: &DirEntry) {
    #[allow(clippy::option_if_let_else)]
    let html_file = match entry.file_name().to_str() {
        Some(fname) => {
            let mut tmp_path = PathBuf::from(fname);
            tmp_path.set_file_name("index.html");
            tmp_path
        }
        None => panic!("Could not write html file {}", entry.path().display()),
    };
    let file_path = target_dir.join(html_file.as_path());
    let result = File::create(file_path.as_path()).and_then(|mut f| f.write_all(html.as_bytes()));
    assert!(
        result.is_ok(),
        "Could not write html file {}: {:?}",
        file_path.as_path().display(),
        result.err()
    );
    log::info!("Rendered html {}", file_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remove_numbered_prefix_default_config() {
        let conf = Config::default();
        assert_eq!(remove_number_prefix("name", &conf), "name");
        assert_eq!(remove_number_prefix("foo_name", &conf), "foo_name");
        assert_eq!(remove_number_prefix("foo_1_name", &conf), "foo_1_name");
        assert_eq!(remove_number_prefix("1_", &conf), "1_");
        assert_eq!(remove_number_prefix("_name", &conf), "_name");
        assert_eq!(remove_number_prefix("1_name", &conf), "name");
        assert_eq!(remove_number_prefix("12_name", &conf), "name");
        assert_eq!(remove_number_prefix("123_name", &conf), "name");
    }

    #[test]
    fn remove_numbered_prefix_explicit_true() {
        let conf = Config {
            remove_numbered_prefix: Some(true),
            ..Default::default()
        };
        assert_eq!(remove_number_prefix("name", &conf), "name");
        assert_eq!(remove_number_prefix("foo_name", &conf), "foo_name");
        assert_eq!(remove_number_prefix("foo_1_name", &conf), "foo_1_name");
        assert_eq!(remove_number_prefix("1_", &conf), "1_");
        assert_eq!(remove_number_prefix("_name", &conf), "_name");
        assert_eq!(remove_number_prefix("1_name", &conf), "name");
        assert_eq!(remove_number_prefix("12_name", &conf), "name");
        assert_eq!(remove_number_prefix("123_name", &conf), "name");
    }

    #[test]
    fn remove_numbered_prefix_explicit_false() {
        let conf = Config {
            remove_numbered_prefix: Some(false),
            ..Default::default()
        };
        assert_eq!(remove_number_prefix("name", &conf), "name");
        assert_eq!(remove_number_prefix("foo_name", &conf), "foo_name");
        assert_eq!(remove_number_prefix("foo_1_name", &conf), "foo_1_name");
        assert_eq!(remove_number_prefix("1_", &conf), "1_");
        assert_eq!(remove_number_prefix("_name", &conf), "_name");
        assert_eq!(remove_number_prefix("1_name", &conf), "1_name");
        assert_eq!(remove_number_prefix("12_name", &conf), "12_name");
        assert_eq!(remove_number_prefix("123_name", &conf), "123_name");
    }
}
