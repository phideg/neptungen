use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub title: Option<String>,
    pub template_dir: Option<String>,
    pub output_dir: Option<String>,
    pub copy_dirs: Option<Vec<String>>,
    pub gallery: Option<Gallery>,
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
        if !self.gallery.is_some() {
            self.gallery = Some(Gallery {
                img_width: 600,
                img_height: 800,
                thumb_width: 90,
                thumb_height: 90,
            })
        }
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
                    panic!("The directory '{}' does not exist in your project '{}'",
                           copy_dir.as_str(),
                           base_path.display());
                }
                new_copy_dirs.push(path.as_path()
                    .to_str()
                    .expect("Could not resolve path to copy directory")
                    .to_owned());
            }
            self.copy_dirs = Some(new_copy_dirs);
        }
    }

    pub fn print(&self) {
        println!("Title : {:?}", self.title);
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
        if self.gallery.is_some() {
            let gallery = self.gallery.as_ref().unwrap();
            println!("Gallery");
            println!("  image size : {} x {}",
                     gallery.img_width,
                     gallery.img_height);
            println!("  thumb size : {} x {}",
                     gallery.thumb_width,
                     gallery.thumb_height);
        }
    }
}
