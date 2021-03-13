use crate::config::Config;
use crate::errors::*;
use crate::filter::{
    contains_markdown_file, contains_markdown_subdir, is_directory, is_hidden, is_image,
    is_markdown,
};
use crate::template;
use image;
use liquid;
use pulldown_cmark::{html, Options, Parser};
use rayon::prelude::*;
use std::fmt::{self, Debug};
use std::fs;
use std::fs::DirBuilder;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use walkdir::{DirEntry, WalkDir};

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

pub fn build(path: &Path, conf: &Config) -> Result<()> {
    let path_comps = path.components().collect::<Vec<_>>();
    let output_dir = PathBuf::from(conf.output_dir.as_ref().unwrap());
    let nav_items = prepare_site_structure(path, output_dir.as_path());
    // let last_gen_timestmp = set_and_determine_last_generation(output_dir.as_path());
    let entries = WalkDir::new(path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| is_hidden(e))
        .filter(|e| e.is_ok() && is_markdown(e.as_ref().unwrap()))
        .collect::<Vec<_>>();
    entries.par_iter().for_each(|e| {
        let src = e.as_ref().unwrap();
        let mut target_dir = output_dir.clone();
        if src.path().parent().is_some() {
            for comp in src
                .path()
                .parent()
                .unwrap()
                .components()
                .skip(path_comps.len())
            {
                target_dir.push(comp.as_os_str());
            }
        }
        build_page(nav_items.clone(), src, target_dir.as_path(), conf);
    });
    copy_dirs(path, output_dir.as_path(), conf);
    Ok(())
}

fn build_page(
    nav_items: Vec<liquid::model::value::Value>,
    entry: &DirEntry,
    target_dir: &Path,
    conf: &Config,
) {
    let page_content = convert_markdown_to_html(entry.path());
    let html = if entry.file_name() == "gallery.md" {
        let images = prepare_gallery(entry, target_dir, conf);
        apply_gallery_template(&page_content, nav_items, images, entry.depth(), conf)
    } else {
        apply_page_template(&page_content, nav_items, entry.depth(), conf)
    };
    write_html_file(&html, target_dir, entry);
    copy_images(entry.path().parent().unwrap(), target_dir);
}

fn copy_dirs(path: &Path, target_path: &Path, conf: &Config) {
    if conf.copy_dirs.is_some() {
        let path_comps = path.components().collect::<Vec<_>>();
        for copy_dir in conf.copy_dirs.as_ref().unwrap() {
            let walker = WalkDir::new(path.join(copy_dir).as_path())
                .min_depth(1)
                .into_iter();
            for entry in walker.filter(|e| e.is_ok() && !is_directory(e.as_ref().unwrap())) {
                let entry = entry.unwrap();
                let mut target_file = PathBuf::from(target_path);
                for comp in entry.path().components().skip(path_comps.len()) {
                    target_file.push(comp.as_os_str());
                }
                match DirBuilder::new()
                    .recursive(true)
                    .create(target_file.parent().expect("Missing parent folder!"))
                {
                    Ok(_) => {}
                    Err(e) => println!("{}", e),
                }
                if !target_file.exists() {
                    fs::copy(entry.path(), target_file)
                        .expect(format!("error during copy of {:?}", copy_dir).as_ref());
                }
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
    for entry in walker.filter(|e| e.is_ok() && is_image(e.as_ref().unwrap())) {
        let mut target_file = PathBuf::from(target);
        let entry = entry.unwrap();
        target_file.push(entry.path().file_name().unwrap());
        if !target_file.exists() {
            fs::copy(entry.path(), target_file.as_path())
                .expect(format!("Error during copy of {:?}", entry.path().display()).as_ref());
        }
    }
}

fn prepare_site_structure(path: &Path, target_path: &Path) -> Vec<liquid::model::value::Value> {
    let mut nav_entries = Vec::<liquid::model::value::Value>::new();
    let walker = WalkDir::new(path)
        .min_depth(1)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()))
        .into_iter();
    let mut prev_depth = 1;
    let path_comps = path.components().collect::<Vec<_>>();
    for entry in
        walker.filter_entry(|e| is_hidden(e) && is_directory(e) && contains_markdown_file(e))
    {
        let entry = entry.expect("Reading directory entry failed");
        let name = String::from(
            entry
                .file_name()
                .to_str()
                .expect("Failed to read navigation entries"),
        );
        let mut url = PathBuf::new();
        for comp in entry.path().components().skip(path_comps.len()) {
            url.push(comp.as_os_str());
        }
        let target_dir = target_path.join(url.as_path());
        match DirBuilder::new().recursive(true).create(target_dir) {
            Ok(_) => {}
            Err(e) => println!("{}", e),
        }
        url.push("index.html");
        let (menu_cmd, level_depth) =
            match (contains_markdown_subdir(&entry), prev_depth > entry.depth()) {
                (true, true) => (MenuCmd::CloseOpenLevel, prev_depth - entry.depth()),
                (true, false) => (MenuCmd::OpenLevel, 0),
                (false, true) => (MenuCmd::CloseLevel, prev_depth - entry.depth()),
                _ => (MenuCmd::None, 0),
            };
        let nav_entry = liquid::object!({
            "name": name,
            "url" : url.as_os_str().to_str().unwrap().to_owned(),
            "menu_cmd" : menu_cmd.to_string().to_owned(),
            "level_depth" : level_depth as i32,
        });
        nav_entries.push(liquid::model::value::Value::Object(nav_entry));
        prev_depth = entry.depth();
    }
    nav_entries
}

fn prepare_gallery(
    source_entry: &DirEntry,
    target_path: &Path,
    conf: &Config,
) -> Vec<liquid::model::value::Value> {
    let gallery_settings = conf.gallery.as_ref().unwrap();
    let mut images = Vec::<liquid::model::value::Value>::new();
    let img_dir = gallery_settings.img_dir.as_ref().unwrap();
    let target_dir = target_path.join(img_dir.as_str());
    match DirBuilder::new().recursive(true).create(&target_dir) {
        Ok(_) => {}
        Err(e) => println!("{}", e),
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
        let mut img = image::open(entry.path()).expect(
            format!(
                "Resize of '{}' failed: The gallery folder should only contain \
                 images!",
                entry.path().display()
            )
            .as_ref(),
        );

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
                .expect(format!("Saving image '{}' failed", image_path.display()).as_ref());
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
                .expect(format!("Saving thumb image '{}' failed", thumb_path.display()).as_ref());
        }

        let image_entry = liquid::object!({
            "name"  : rel_image_path.to_str().unwrap().to_owned(),
            "thumb" : rel_thumb_path.to_str().unwrap().to_owned(),
        });
        images.push(liquid::model::value::Value::Object(image_entry));
    }
    images
}

fn apply_gallery_template(
    content: &str,
    nav_items: Vec<liquid::model::value::Value>,
    images: Vec<liquid::model::value::Value>,
    depth: usize,
    conf: &Config,
) -> String {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .unwrap()
        .parse(template::load_gallery_template(conf).as_str())
        .expect("Gallery template could not be parsed!");
    let mut root_dir = String::new();
    for _ in 1..depth {
        root_dir.push_str("../");
    }
    let mut context = liquid::model::Object::new();
    context.insert(
        "root_dir".into(),
        liquid::model::value::Value::scalar(root_dir),
    );
    context.insert(
        "title".into(),
        liquid::model::value::Value::scalar(if conf.title.is_some() {
            conf.title.as_ref().unwrap().clone()
        } else {
            "None".to_string()
        }),
    );
    context.insert(
        "nav_items".into(),
        liquid::model::value::Value::Array(nav_items),
    );
    context.insert(
        "content".into(),
        liquid::model::value::Value::scalar(content.to_owned()),
    );
    context.insert("images".into(), liquid::model::value::Value::Array(images));
    match template.render(&context) {
        Ok(output) => output,
        Err(error) => panic!("Could not render Page template: {}", error),
    }
}

fn apply_page_template(
    content: &str,
    nav_items: Vec<liquid::model::value::Value>,
    depth: usize,
    conf: &Config,
) -> String {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .unwrap()
        .parse(template::load_page_template(conf).as_str())
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
        "nav_items" : liquid::model::value::Value::Array(nav_items),
        "content" : content.to_owned(),
    });
    match template.render(&context) {
        Ok(output) => output,
        Err(error) => panic!("Could not render Page template: {}", error),
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
    if result.is_err() {
        panic!(
            "Could not write html file {}: {:?}",
            file_path.as_path().display(),
            result.err()
        );
    }
    println!("written file {}", file_path.display());
}
