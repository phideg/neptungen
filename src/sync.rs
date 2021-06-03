use crate::comp_as_str;
use crate::config::{Config, FtpProtocol};
use crate::ftp::{Ftp, FtpOperations, Sftp};
use crate::{filter, last_path_comp_as_str};
use anyhow::{anyhow, Context, Result};
use checksums::ops;
use checksums::ops::{CompareFileResult, CompareResult};
use checksums::Algorithm;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{stderr, stdout};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

static CRC_FILE_NAME: &str = "hashsums.crc";
static OLD_CRC_FILE_NAME: &str = "hashsums_old.crc";

pub struct Synchronizer {
    server: String,
    ftp_target_dir: PathBuf,
    hashsums: BTreeMap<String, String>,
    output_path: PathBuf,
    output_path_offset: usize,
    ftp_stream: Box<dyn FtpOperations>,
}

impl Synchronizer {
    pub fn new(conf: &Config) -> Result<Synchronizer> {
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
        let port = sync_settings.ftp_port.unwrap_or(21);
        let protocol = sync_settings.ftp_protocol.unwrap_or(FtpProtocol::Ftp);
        let ftp_stream: Box<dyn FtpOperations> = if protocol == FtpProtocol::Ftp {
            Box::new(Ftp::new(&server, port, &user)?)
        } else {
            Box::new(Sftp::new(&server, port, &user)?)
        };
        Ok(Synchronizer {
            server,
            ftp_target_dir: PathBuf::from(sync_settings.ftp_target_dir.as_deref().unwrap_or("/")),
            hashsums: ops::create_hashes(
                &output_path,
                BTreeSet::new(),
                Algorithm::CRC64,
                None,
                false,
                4,
                stdout(),
                &mut stderr(),
            ),
            output_path_offset: output_path.as_path().components().count(),
            output_path,
            ftp_stream,
        })
    }

    pub fn execute(&mut self) -> Result<()> {
        self.create_checksums_file();

        // first try to push delta first
        if let Err(err) = self.push_delta() {
            println!("Couldn't push delta '{}' try to push all instead!", err);
            self.push_all_files()?;
        }
        let crc_file_path = self.output_path.join(CRC_FILE_NAME);
        self.push_file(&crc_file_path)?;
        Ok(())
    }

    pub fn push_all_files(&mut self) -> Result<()> {
        println!("Push all files to FTP server {}", &self.server);
        let walker = WalkDir::new(self.output_path.as_path())
            .min_depth(1)
            .into_iter();
        for entry in walker {
            let entry = entry.unwrap();
            if filter::is_directory(&entry) {
                self.create_and_change_to_directory(entry.path())?;
            } else {
                self.push_file(entry.path())?;
            }
        }
        Ok(())
    }

    fn create_checksums_file(&self) {
        let hash_file_path = PathBuf::from(&self.output_path).join(CRC_FILE_NAME);
        let hash_file = hash_file_path.to_string_lossy().to_string();
        ops::write_hashes(
            &(hash_file, hash_file_path),
            Algorithm::CRC64,
            self.hashsums.clone(),
        );
    }

    fn retrieve_checksums_file(&mut self) -> Result<PathBuf> {
        let mut hash_file_path = self.create_and_change_to_root()?;
        hash_file_path.push(CRC_FILE_NAME);
        let old_hash_file_path = PathBuf::from(&self.output_path).join(OLD_CRC_FILE_NAME);
        self.ftp_stream
            .get(&hash_file_path, &old_hash_file_path)
            .context("loading of existing checksums file failed")?;
        Ok(old_hash_file_path)
    }

    fn push_delta(&mut self) -> Result<()> {
        // load old checksums
        let old_hash_file_path = self.retrieve_checksums_file()?;
        let old_hash_file = old_hash_file_path.to_string_lossy().to_string();
        let old_hashsums =
            ops::read_hashes(&mut stderr(), &(old_hash_file, old_hash_file_path.clone()))
                .map_err(|_| anyhow!("Couldn't read existing checksums"))?;
        // ignore if old CRC file could not be deleted
        let _ = fs::remove_file(&old_hash_file_path);
        // compare checksums
        let (compare_result, compare_file_result) =
            ops::compare_hashes(CRC_FILE_NAME, self.hashsums.clone(), old_hashsums)
                .map_err(|_| anyhow!("Couldn't compare checksum files"))?;
        for entry in compare_result.into_iter().skip(1) {
            match entry {
                CompareResult::FileRemoved(file) => {
                    println!("Removing file {:?}", file);
                    let path = self.output_path.join(file.as_str());
                    if path.is_dir() {
                        self.remove_directory(&path)?;
                    } else {
                        self.remove_file(&path)?;
                    }
                }
                CompareResult::FileAdded(file) => {
                    println!("Adding file {:?}", file.as_str());
                    let path = self.output_path.join(file.as_str());
                    if path.is_dir() {
                        self.create_and_change_to_directory(path.as_path())?;
                    } else {
                        self.push_file(path.as_path())?;
                    }
                }
                _ => {}
            }
        }
        for entry in compare_file_result {
            if let CompareFileResult::FileDiffers { file, .. } = entry {
                println!("Updateing file {:?}", file);
                let path = self.output_path.join(file.as_str());
                if path.is_dir() {
                    self.create_and_change_to_directory(path.as_path())?;
                } else {
                    self.push_file(path.as_path())?;
                }
            }
        }
        Ok(())
    }

    fn create_and_change_to_root(&mut self) -> Result<PathBuf> {
        let mut to_dir = PathBuf::new();
        for comp in self.ftp_target_dir.components() {
            let new_dir = comp_as_str!(comp)?;
            to_dir.push(new_dir);
            if self.ftp_stream.cwd(&to_dir).is_err() {
                println!("Creating ftp target dir '{}'", new_dir);
                self.ftp_stream.mkdir(&to_dir)?;
                self.ftp_stream.cwd(&to_dir)?;
            }
        }
        Ok(to_dir)
    }

    fn create_and_change_to_directory(&mut self, dir: &Path) -> Result<PathBuf> {
        let mut to_dir = self.create_and_change_to_root()?;
        for comp in dir.components().skip(self.output_path_offset) {
            let new_dir = comp_as_str!(comp)?;
            to_dir.push(new_dir);
            if self.ftp_stream.cwd(&to_dir).is_err() {
                println!("Creating '{}'", new_dir);
                self.ftp_stream.mkdir(&to_dir)?;
                self.ftp_stream.cwd(&to_dir)?;
            }
        }
        Ok(to_dir)
    }

    fn cd_to_parent_dir(&mut self, src: &Path) -> Result<PathBuf> {
        let mut parent_dir = PathBuf::from(src);
        if !parent_dir.pop() {
            return Err(anyhow!(
                "Internal error: couldn't handle path {:?}",
                parent_dir
            ));
        }
        let mut path = self.create_and_change_to_root()?;
        for comp in parent_dir.components().skip(self.output_path_offset) {
            path.push(comp_as_str!(comp)?);
            self.ftp_stream.cwd(&path)?;
        }
        Ok(path)
    }

    fn remove_directory(&mut self, dir: &Path) -> Result<()> {
        let mut src_dir = self.cd_to_parent_dir(dir)?;
        src_dir.push(last_path_comp_as_str!(dir)?);
        self.ftp_stream.rmdir(&src_dir)?;
        Ok(())
    }

    fn remove_file(&mut self, file: &Path) -> Result<()> {
        let mut src_dir = self.cd_to_parent_dir(file)?;
        src_dir.push(last_path_comp_as_str!(file)?);
        self.ftp_stream.del(&src_dir)?;
        Ok(())
    }

    fn push_file(&mut self, file: &Path) -> Result<()> {
        let mut to_dir = self.create_and_change_to_directory(file.parent().unwrap())?;
        let file_name = file.file_name().and_then(|s| s.to_str()).unwrap();
        to_dir.push(file_name);
        self.ftp_stream.put(&to_dir, file)
    }
}
