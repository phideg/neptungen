use crate::config::Config;
use crate::filter;
use anyhow::{anyhow, Context, Result};
use checksums::ops;
use checksums::ops::{CompareFileResult, CompareResult};
use checksums::Algorithm;
use ftp::types::FileType;
use ftp::FtpError;
use ftp::FtpStream;
use rpassword;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{stderr, stdout};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

static CRC_FILE_NAME: &str = "hashsums.crc";
static OLD_CRC_FILE_NAME: &str = "hashsums_old.crc";

pub struct Synchronizer<'a> {
    conf: &'a Config,
    hashsums: BTreeMap<String, String>,
    output_path: PathBuf,
    output_path_offset: usize,
    ftp_stream: FtpStream,
}

impl<'a> Synchronizer<'a> {
    pub fn new(conf: &'a Config) -> Result<Synchronizer> {
        let output_path = PathBuf::from(
            conf.output_dir
                .as_ref()
                .context("check 'config.toml' output_dir is invalid")?,
        );
        let output_path_comps = output_path.as_path().components().collect::<Vec<_>>();
        let ftp_stream = Synchronizer::ftp_login(conf)?;
        Ok(Synchronizer {
            conf,
            hashsums: ops::create_hashes(
                output_path.as_path(),
                BTreeSet::new(),
                Algorithm::CRC64,
                None,
                false,
                4,
                stdout(),
                &mut stderr(),
            ),
            output_path: output_path.clone(),
            output_path_offset: output_path_comps.len(),
            ftp_stream,
        })
    }

    fn ftp_login(conf: &Config) -> Result<FtpStream> {
        // connect to ftp server
        let sync_settings = conf
            .sync_settings
            .as_ref()
            .context("'config.toml' sync settings not found")?;
        let connection_str = format!(
            "{}:{}",
            sync_settings.ftp_server,
            sync_settings.ftp_port.unwrap_or(21)
        );
        let mut ftp_stream = FtpStream::connect(connection_str.as_str())?;
        println!(
            "Enter password for ftp server '{}' user '{}'",
            sync_settings.ftp_server.as_str(),
            sync_settings.ftp_user.as_str()
        );
        let passwd = rpassword::prompt_password_stdout("Password: ")
            .context("Couldn't read password from standard input")?;
        ftp_stream
            .login(sync_settings.ftp_user.as_str(), passwd.as_str())
            .context("FTP Login failed.")?;
        Ok(ftp_stream)
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
        let old_hash_file_path = PathBuf::from(&self.output_path).join(OLD_CRC_FILE_NAME);
        self.ftp_stream
            .transfer_type(FileType::Binary)
            .context("FTP couldn't switch to binary mode.")?;
        self.ftp_stream
            .retr(CRC_FILE_NAME, |stream| {
                let mut buf = Vec::new();
                stream
                    .read_to_end(&mut buf)
                    .map(|_| {
                        let mut f = File::create(old_hash_file_path.as_path()).expect(
                            format!("Couldn't create file '{}'.", OLD_CRC_FILE_NAME).as_str(),
                        );
                        f.write_all(buf.as_slice())
                            .expect("Couldn't write hashsums file.")
                    })
                    .map_err(FtpError::ConnectionError)
            })
            .context("loading of existing checksums file failed")?;
        Ok(old_hash_file_path)
    }

    fn push_delta(&mut self) -> Result<()> {
        // load old checksums
        let old_hash_file_path = self.retrieve_checksums_file()?;
        let old_hash_file = old_hash_file_path.to_string_lossy().to_string();
        let old_hashsums = ops::read_hashes(&mut stderr(), &(old_hash_file, old_hash_file_path))
            .map_err(|_| anyhow!("Couldn't read existing checksums"))?;
        // ignore if old CRC file could not be deleted
        let _ = fs::remove_file(self.output_path.join(OLD_CRC_FILE_NAME).as_path());
        // compare checksums
        let (compare_result, compare_file_result) =
            ops::compare_hashes(CRC_FILE_NAME, self.hashsums.clone(), old_hashsums)
                .map_err(|_| anyhow!("Couldn't compare checksum files"))?;
        for entry in compare_result {
            match entry {
                CompareResult::FileRemoved(file) => {
                    println!("Removing file {:?}", file);
                    let path = self.output_path.join(file.as_str());
                    if path.is_dir() {
                        self.remove_directory(path.as_path())?;
                    } else {
                        self.remove_file(path.as_path())?;
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

    pub fn execute(&mut self) -> Result<()> {
        self.create_checksums_file();

        // first try to push delta first
        if let Err(err) = self.push_delta() {
            println!("Couldn't push delta '{}' try to push all instead!", err);
            self.push_all_files()?;
        }
        let crc_file_path = self.output_path.join(CRC_FILE_NAME);
        self.push_file(crc_file_path.as_path())?;
        Ok(())
    }

    pub fn push_all_files(&mut self) -> Result<()> {
        println!(
            "Push all files to FTP server {}",
            self.conf
                .sync_settings
                .as_ref()
                .unwrap()
                .ftp_server
                .as_str()
        );
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

    fn create_and_change_to_directory(&mut self, src_dir: &Path) -> Result<()> {
        self.ftp_stream
            .cwd(
                self.conf
                    .sync_settings
                    .as_ref()
                    .unwrap()
                    .ftp_target_dir
                    .as_deref()
                    .unwrap_or("/"),
            )
            .context("Couldn't change to root dir")?;
        for comp in src_dir.components().skip(self.output_path_offset) {
            let dir = comp
                .as_os_str()
                .to_str()
                .with_context(|| format!("Couldn't convert component '{:?}' to string.", comp))?;
            if self.ftp_stream.cwd(dir).is_err() {
                println!("Creating '{}'", dir);
                self.ftp_stream
                    .mkdir(dir)
                    .with_context(|| format!("Couldn't create dir '{}'", &dir))?;
                self.ftp_stream
                    .cwd(dir)
                    .with_context(|| format!("Couldn't change to dir '{}'", &dir))?;
            }
        }
        Ok(())
    }

    fn change_to_dir(&mut self, to_dir: &Path)  -> Result<()> {
        self.ftp_stream
            .cwd("/")
            .context("Couldn't change to root dir")?;
        for comp in to_dir.components().skip(self.output_path_offset) {
            let dir = comp
                .as_os_str()
                .to_str()
                .with_context(|| format!("Couldn't convert component '{:?}' to string.", comp))?;
            self.ftp_stream.cwd(dir).with_context(|| format!("FTP: couldn't change to '{}'", dir))?;
        }
        Ok(())
    }

    fn remove_directory(&mut self, src_dir: &Path) -> Result<()> {
        self.change_to_dir(src_dir)?;
        let comp = src_dir.components().last().unwrap();
        if self.ftp_stream.cwd("..").is_ok() {
            self.ftp_stream
                .rmdir(comp.as_os_str().to_str().unwrap())
                .with_context(|| format!("Couldn't remove dir '{:?}'", src_dir))?;
        }
        Ok(())
    }

    fn remove_file(&mut self, src_dir: &Path) -> Result<()> {
        self.change_to_dir(src_dir)?;
        let comp = src_dir.components().last().unwrap();
        self.ftp_stream
            .rm(comp.as_os_str().to_str().unwrap())
            .expect(format!("Couldn't remove file '{:?}'", &comp).as_ref());
        Ok(())
    }

    fn push_file(&mut self, src_path: &Path) -> Result<()> {
        self.create_and_change_to_directory(src_path.parent().unwrap())?;
        let file_name = src_path.file_name().and_then(|s| s.to_str()).unwrap();
        let mut f = File::open(src_path)?;
        self.ftp_stream.put(file_name, &mut f).with_context(|| format!("Couldn't push file '{}' to ftp server", file_name))
    }
}

impl<'a> Drop for Synchronizer<'a> {
    fn drop(&mut self) {
        let _ = self.ftp_stream.quit();
    }
}
