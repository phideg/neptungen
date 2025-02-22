use crate::config::{Config, FtpProtocol};
use crate::ftp::{Ftp, Operations, Sftp};
use crate::{comp_as_str, sha1dir};
use crate::{filter, last_path_comp_as_str};

use anyhow::{Context, Result, anyhow};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

static CRC_FILE_NAME: &str = "checksums.crc";
static OLD_CRC_FILE_NAME: &str = "checksums_old.crc";

pub struct Synchronizer {
    server: String,
    ftp_target_dir: PathBuf,
    output_path: PathBuf,
    output_path_offset: usize,
    ftp_stream: Box<dyn Operations>,
}

impl Synchronizer {
    pub fn new(conf: &Config) -> Result<Self> {
        let output_path = PathBuf::from(
            conf.output_dir
                .as_ref()
                .context("check 'config.toml' output_dir is invalid")?,
        );
        let sync_settings = conf
            .sync_settings
            .as_ref()
            .context("'config.toml' sync settings not found")?;
        let server = String::from(&sync_settings.ftp_server);
        let user = &sync_settings.ftp_user;
        let protocol = sync_settings.ftp_protocol.unwrap_or(FtpProtocol::Ftp);
        let port = sync_settings
            .ftp_port
            .unwrap_or(if protocol == FtpProtocol::Ftp { 21 } else { 22 });
        let ftp_stream: Box<dyn Operations> = if protocol == FtpProtocol::Ftp {
            Box::new(Ftp::new(&server, port, user)?)
        } else {
            Box::new(Sftp::new(&server, port, user)?)
        };
        Ok(Self {
            server,
            ftp_target_dir: PathBuf::from(sync_settings.ftp_target_dir.as_deref().unwrap_or("/")),
            output_path_offset: output_path.as_path().components().count(),
            output_path,
            ftp_stream,
        })
    }

    pub fn execute(&mut self) -> Result<()> {
        let checksums = self.create_checksums_file()?;

        // first try to push delta first
        if let Err(ref err) = self.push_delta(&checksums) {
            println!("Couldn't push delta '{err}'");
            log::info!("Couldn't push delta '{err}'");
            self.push_all_files()?;
        }
        let crc_file_path = self.output_path.join(CRC_FILE_NAME);
        self.ftp_push_file(&crc_file_path)?;
        Ok(())
    }

    pub fn push_all_files(&mut self) -> Result<()> {
        println!("Push all files to FTP server {}", &self.server);
        log::info!("Push all files to FTP server {}", &self.server);
        let walker = WalkDir::new(self.output_path.as_path())
            .min_depth(1)
            .into_iter();
        for entry in walker {
            let entry = entry.unwrap();
            if filter::is_directory(&entry) {
                self.ftp_create_and_goto_dir(entry.path())?;
            } else {
                self.ftp_push_file(entry.path())?;
            }
        }
        Ok(())
    }

    fn create_checksums_file(&self) -> Result<BTreeMap<PathBuf, [u8; 20]>> {
        let hash_file_path = PathBuf::from(&self.output_path).join(CRC_FILE_NAME);
        std::fs::remove_file(&hash_file_path).ok(); // ignore if checksums did not exist before
        let checksums = sha1dir::create_hashes(&self.output_path)?;
        let f = std::fs::File::create(&hash_file_path).expect("Unable to create file");
        serde_json::to_writer(f, &checksums).expect("Unable to write data");
        Ok(checksums)
    }

    fn retrieve_checksums_file(&mut self) -> Result<PathBuf> {
        let mut hash_file_path = self.ftp_goto_target_dir()?;
        hash_file_path.push(CRC_FILE_NAME);
        let old_hash_file_path = PathBuf::from(&self.output_path).join(OLD_CRC_FILE_NAME);
        self.ftp_stream
            .get(&hash_file_path, &old_hash_file_path)
            .context("loading of existing checksums file failed")?;
        Ok(old_hash_file_path)
    }

    fn push_delta(&mut self, checksums: &BTreeMap<PathBuf, [u8; 20]>) -> Result<()> {
        // load old checksums
        let old_hash_file_path = self.retrieve_checksums_file()?;
        let mut data = String::new();
        std::fs::File::open(&old_hash_file_path)
            .and_then(|mut f| f.read_to_string(&mut data))
            .context("couldn't find or read file 'config.toml'")?;
        let mut old_checksums: BTreeMap<PathBuf, [u8; 20]> = serde_json::from_str(&data)?;

        // convert Result to Option in order to ignore if the old CRC file could not be deleted
        fs::remove_file(&old_hash_file_path).ok();

        // check for deltas
        let mut parent_dir: Option<&Path> = None;
        let mut new_dir_found = false;
        for (new_path, new_hash) in checksums {
            // copy new directory
            if new_dir_found && new_path.starts_with(parent_dir.unwrap()) {
                log::info!("Created: {new_path:?}");
                if new_path.is_dir() {
                    self.ftp_create_and_goto_dir(new_path)?;
                } else {
                    self.ftp_push_file(new_path)?;
                }
                continue;
            }
            new_dir_found = false;

            // skip unchanged entries
            if let Some(d) = parent_dir {
                if new_path.starts_with(d) {
                    log::info!("Unchanged: {new_path:?}");
                    old_checksums.remove(new_path);
                    continue;
                }
                parent_dir = None;
            }

            if let Some(old_hash) = old_checksums.remove(new_path) {
                // entry exists on remote check wether update is necessary
                let hash_is_equal = &old_hash == new_hash;
                if hash_is_equal {
                    log::info!("Unchanged: {new_path:?}");
                    if new_path.is_dir() {
                        parent_dir = Some(new_path);
                    } else {
                        continue;
                    }
                } else {
                    log::info!("Updated: {new_path:?}");
                    if new_path.is_dir() {
                        self.ftp_create_and_goto_dir(new_path)?;
                    } else {
                        self.ftp_push_file(new_path)?;
                    }
                }
            } else {
                // entry should not exist on remote but maybe the path still exists
                // therefore delete file or directory to be sure!
                if self.ftp_remove_file(new_path).is_ok() {
                    log::error!("Removed?: {new_path:?} - maybe changed file into dir?");
                } else if self.ftp_remove_dir(new_path).is_ok() {
                    log::error!("Removed?: {new_path:?} - maybe changed dir to filename?");
                }
                // now we can safely update the remote
                log::info!("Created: {new_path:?}");
                if new_path.is_dir() {
                    self.ftp_create_and_goto_dir(new_path)?;
                    parent_dir = Some(new_path);
                    new_dir_found = true;
                } else {
                    self.ftp_push_file(new_path)?;
                }
            }
        }

        // finally delete removed entries from remote
        parent_dir = None;
        for old_path in old_checksums.keys() {
            log::info!("removed {old_path:?}");
            if self.ftp_is_dir(old_path) {
                if let Some(del_dir) = parent_dir {
                    self.ftp_remove_dir(del_dir)?;
                }
                parent_dir = Some(old_path);
            } else {
                self.ftp_remove_file(old_path)?;
            }
        }
        if let Some(del_dir) = parent_dir {
            self.ftp_remove_dir(del_dir)?;
        }

        Ok(())
    }

    fn ftp_goto_target_dir(&mut self) -> Result<PathBuf> {
        self.ftp_stream.cwdroot()?;
        let mut to_dir = PathBuf::new();
        for comp in self
            .ftp_target_dir
            .components()
            .skip_while(|c| matches!(*c, Component::Prefix(_) | Component::RootDir))
        {
            let new_dir = comp_as_str!(comp)?;
            to_dir.push(new_dir);
            if self.ftp_stream.cwd(&to_dir).is_err() {
                log::info!("Creating '{new_dir}'");
                self.ftp_stream.mkdir(&to_dir)?;
                self.ftp_stream.cwd(&to_dir)?;
            }
        }
        Ok(to_dir)
    }

    fn ftp_create_and_goto_dir(&mut self, dir: &Path) -> Result<PathBuf> {
        let mut to_dir = self.ftp_goto_target_dir()?;
        for comp in dir.components().skip(if dir.starts_with(".") {
            1
        } else {
            self.output_path_offset
        }) {
            let new_dir = comp_as_str!(comp)?;
            to_dir.push(new_dir);
            if self.ftp_stream.cwd(&to_dir).is_err() {
                log::info!("Creating '{new_dir}'");
                self.ftp_stream.mkdir(&to_dir)?;
                self.ftp_stream.cwd(&to_dir)?;
            }
        }
        Ok(to_dir)
    }

    fn ftp_goto_parent_dir(&mut self, src: &Path) -> Result<PathBuf> {
        let mut parent_dir = src.to_path_buf();
        if !parent_dir.pop() {
            return Err(anyhow!(
                "Internal error: couldn't handle path {:?}",
                parent_dir
            ));
        }
        self.ftp_goto_dir(parent_dir.as_path())
    }

    fn ftp_goto_dir(&mut self, src: &Path) -> Result<PathBuf> {
        let mut path = self.ftp_goto_target_dir()?;
        for comp in src.components().skip(if src.starts_with(".") {
            1
        } else {
            self.output_path_offset
        }) {
            path.push(comp_as_str!(comp)?);
            self.ftp_stream.cwd(&path)?;
        }
        Ok(path)
    }

    fn ftp_remove_dir(&mut self, dir: &Path) -> Result<()> {
        let mut src_dir = self.ftp_goto_parent_dir(dir)?;
        src_dir.push(last_path_comp_as_str!(dir)?);
        self.ftp_stream.rmdir(&src_dir)?;
        Ok(())
    }

    fn ftp_remove_file(&mut self, file: &Path) -> Result<()> {
        let mut src_dir = self.ftp_goto_parent_dir(file)?;
        src_dir.push(last_path_comp_as_str!(file)?);
        self.ftp_stream.del(&src_dir)?;
        Ok(())
    }

    fn ftp_push_file(&mut self, file: &Path) -> Result<()> {
        let mut to_dir = self.ftp_create_and_goto_dir(file.parent().unwrap())?;
        let file_name = file.file_name().and_then(OsStr::to_str).unwrap();
        to_dir.push(file_name);
        self.ftp_stream.put(&to_dir, file)
    }

    fn ftp_is_dir(&mut self, path: &Path) -> bool {
        self.ftp_goto_dir(path).is_ok()
    }
}
