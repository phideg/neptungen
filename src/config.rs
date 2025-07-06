use anyhow::{Context, Result};
use serde_derive::Deserialize;
use std::path::Path;

static GALLERY_FOLDER_NAME: &str = "images";
static OUTPUT_FOLDER_NAME: &str = "_output";

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub title: Option<String>,
    pub template_dir: Option<String>,
    pub output_dir: Option<String>,
    pub logging: Option<LogKind>,
    pub remove_numbered_prefix: Option<bool>,
    pub copy_dirs: Option<Vec<String>>,
    pub gallery: Option<Gallery>,
    pub sync_settings: Option<SyncSettings>,
}

#[derive(Debug, Deserialize)]
pub struct Gallery {
    pub img_dir: Option<String>,
    pub img_width: u32,
    pub img_height: u32,
    pub thumb_width: u32,
    pub thumb_height: u32,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Deserialize)]
pub enum FtpProtocol {
    Ftp,
    Sftp,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug, Deserialize)]
pub enum LogKind {
    Stdout,
    File,
}

#[allow(clippy::struct_field_names)]
#[derive(Debug, Deserialize)]
pub struct SyncSettings {
    pub ftp_server: String,
    pub ftp_port: Option<u32>,
    pub ftp_protocol: Option<FtpProtocol>,
    pub ftp_user: String,
    pub ftp_target_dir: Option<String>,
    pub ftp_overwrite: Option<bool>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let mut input = String::new();
        std::fs::File::open(path.join("config.toml").as_path())
            .and_then(|mut f| std::io::Read::read_to_string(&mut f, &mut input))
            .context("couldn't find or read file 'config.toml'")?;
        let mut conf =
            toml::from_str::<Self>(input.as_str()).context("parsing 'config.toml' failed")?;
        conf.resolve_paths(path);
        Ok(conf)
    }

    pub fn resolve_paths(&mut self, base_path: &Path) {
        let output_path = base_path.join(self.output_dir.as_deref().unwrap_or(OUTPUT_FOLDER_NAME));
        self.output_dir = Some(
            output_path
                .as_path()
                .to_str()
                .expect("Could not resolve path to template directory")
                .to_owned(),
        );
        if self.gallery.is_none() {
            self.gallery = Some(Gallery {
                img_dir: Some(GALLERY_FOLDER_NAME.to_string()),
                img_width: 600,
                img_height: 800,
                thumb_width: 90,
                thumb_height: 90,
            });
        } else if self.gallery.as_ref().unwrap().img_dir.is_none() {
            self.gallery.as_mut().unwrap().img_dir = Some(GALLERY_FOLDER_NAME.to_string());
        }
        if self.template_dir.is_some() {
            let template_path = base_path.join(self.template_dir.as_ref().unwrap().as_str());
            assert!(
                template_path.exists(),
                "The template directory '{}' does not exist in your project '{}'",
                &self.template_dir.as_ref().unwrap().as_str(),
                base_path.display()
            );
            self.template_dir = Some(
                template_path
                    .as_path()
                    .to_str()
                    .expect("Could not resolve path to template directory")
                    .to_owned(),
            );
        }
        if let Some(ref copy_dirs) = self.copy_dirs {
            let mut new_copy_dirs = Vec::<String>::new();
            for copy_dir in copy_dirs {
                let path = base_path.join(copy_dir.as_str());
                assert!(
                    path.exists(),
                    "The directory '{}' does not exist in your project '{}'",
                    copy_dir.as_str(),
                    base_path.display()
                );
                new_copy_dirs.push(
                    path.as_path()
                        .to_str()
                        .expect("Could not resolve path to copy directory")
                        .to_owned(),
                );
            }
            self.copy_dirs = Some(new_copy_dirs);
        }
        if let Some(ref mut sync_settings) = self.sync_settings
            && let Some(ref mut ftp_target_dir) = sync_settings.ftp_target_dir
            && !ftp_target_dir.starts_with('/')
        {
            println!(
                "ftp_target_dir contains no absolute path '/' will be prepended to '{ftp_target_dir}'"
            );
            ftp_target_dir.insert(0, '/');
        }
    }

    pub fn print(&self) {
        use term_painter::Attr::Bold;
        use term_painter::ToStyle;
        let none_string = "None".to_string();
        println!("Title : {}", self.title.as_ref().unwrap_or(&none_string));
        println!(
            "Template directory : {}",
            self.template_dir.as_ref().unwrap_or(&none_string)
        );
        println!(
            "Output directory : {}",
            self.output_dir
                .as_ref()
                .map_or(OUTPUT_FOLDER_NAME, String::as_str)
        );
        println!("{}", Bold.paint("SyncSettings"));
        if self.sync_settings.is_some() {
            let sync_settings = self.sync_settings.as_ref().unwrap();
            println!("  FTP server: {}", sync_settings.ftp_server);
            println!("  FTP port: {}", sync_settings.ftp_port.unwrap_or(21));
            println!("  FTP user: {}", sync_settings.ftp_user);
            println!(
                "  FTP overwrite: {}",
                sync_settings.ftp_overwrite.unwrap_or(false)
            );
        }
        if self.gallery.is_some() {
            let gallery = self.gallery.as_ref().unwrap();
            println!("{}", Bold.paint("Gallery"));
            println!(
                "  image directory: {}",
                gallery
                    .img_dir
                    .as_ref()
                    .map_or(GALLERY_FOLDER_NAME, String::as_str)
            );
            println!(
                "  image size : {} x {}",
                gallery.img_width, gallery.img_height
            );
            println!(
                "  thumb size : {} x {}",
                gallery.thumb_width, gallery.thumb_height
            );
        }
    }
}
