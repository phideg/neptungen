use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub title: String,
    pub template_dir: Option<String>,
    pub output_dir: Option<String>,
    pub copy_dirs: Option<Vec<String>>,
    pub gallery: Gallery,
    pub sync_settings: Option<SyncSettings>,
}

#[derive(Debug, Deserialize)]
pub struct Gallery {
    pub img_width: u32,
    pub img_height: u32,
    pub thumb_width: u32,
    pub thumb_height: u32,
}

#[derive(Debug, Deserialize)]
pub struct SyncSettings {
    pub ftp_server: String,
    pub ftp_port: Option<u32>,
    pub ftp_user: String,
    pub ftp_overwrite: Option<bool>,
}

impl Config {
    pub fn resolve_paths(&mut self, base_path: &Path) {
        let output_path =
            base_path.join(&self.output_dir.as_ref().map(|d| d.as_str()).unwrap_or("_output"));
        self.output_dir = Some(output_path.as_path()
            .to_str()
            .expect("Could not resolve path to template directory")
            .to_owned());
        if self.template_dir.is_some() {
            let template_path = base_path.join(&self.template_dir.as_ref().unwrap().as_str());
            if !template_path.exists() {
                panic!("The template directory '{}' does not exist in your project '{}'",
                       &self.template_dir.as_ref().unwrap().as_str(),
                       base_path.display());
            }
            self.template_dir = Some(template_path.as_path()
                .to_str()
                .expect("Could not resolve path to template directory")
                .to_owned());
        }
        if self.copy_dirs.is_some() {
            let mut new_copy_dirs = Vec::<String>::new();
            for copy_dir in self.copy_dirs.as_ref().unwrap() {
                let path = base_path.join(copy_dir.as_str());
                if !path.exists() {
                    panic!("The template directory '{}' does not exist in your project '{}'",
                           copy_dir.as_str(),
                           base_path.display());
                }
                new_copy_dirs.push(path.as_path()
                    .to_str()
                    .expect("Could not resolve path to template directory")
                    .to_owned());
            }
            self.copy_dirs = Some(new_copy_dirs);
        }
    }

    pub fn print(&self) {
        println!("Title : {}", self.title);
        println!("Template directory : {:?}", self.template_dir);
        println!("Output directory : {:?}", self.output_dir);
        println!("SyncSettings");
        if self.sync_settings.is_some() {
            let sync_settings = self.sync_settings.as_ref().unwrap();
            println!("FTP server: {:?}", sync_settings.ftp_server);
            println!("FTP port: {:?}", sync_settings.ftp_port.unwrap_or(21));
            println!("FTP user: {:?}", sync_settings.ftp_user);
            println!("FTP overwrite: {:?}",
                     sync_settings.ftp_overwrite.unwrap_or(false));
        }
        println!("Gallery");
        println!("  image size : {} x {}",
                 self.gallery.img_width,
                 self.gallery.img_height);
        println!("  thumb size : {} x {}",
                 self.gallery.thumb_width,
                 self.gallery.thumb_height);
    }
}
