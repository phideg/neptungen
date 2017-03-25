use config::Config;
use std::io;
use std::io::prelude::*;
use std::io::{stdout, stderr};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, BTreeSet};
use ftp::FtpStream;
use ftp::types::FileType;
use ftp::FtpError;
use checksums::ops;
use checksums::ops::{CompareResult, CompareFileResult};
use checksums::Algorithm;
use walkdir::WalkDir;
use filter;

static CRC_FILE_NAME: &'static str = "hashsums.crc";
static OLD_CRC_FILE_NAME: &'static str = "hashsums_old.crc";

macro_rules! ftp_stream {
    (&mut self) => (
        self.ftp_stream.expect("FTP not setup correctly")
    )
}

pub struct Synchronizer<'a> {
    conf: &'a Config,
    hashsums: BTreeMap<String, String>,
    output_path: PathBuf,
    output_path_offset: usize,
    ftp_stream: FtpStream,
}

impl<'a> Synchronizer<'a> {
    pub fn new(conf: &'a Config) -> Synchronizer {
        if conf.sync_settings.is_none() {
            panic!("[sync_settings] missing in config.toml.");
        }
        let output_path = PathBuf::from(conf.output_dir.as_ref().unwrap());
        let output_path_comps = output_path.as_path().components().collect::<Vec<_>>();
        let ftp_stream = Synchronizer::ftp_login(conf);
        Synchronizer {
            conf: conf,
            hashsums: ops::create_hashes(output_path.as_path(),
                                         BTreeSet::new(),
                                         Algorithm::CRC64,
                                         None,
                                         false,
                                         4,
                                         stdout(),
                                         &mut stderr()),
            output_path: output_path.clone(),
            output_path_offset: output_path_comps.len(),
            ftp_stream: ftp_stream,
        }
    }

    fn ftp_login(conf: &Config) -> FtpStream {
        // connect to ftp server
        let sync_settings = conf.sync_settings.as_ref().unwrap();
        let connection_str = format!("{}:{}",
                                     sync_settings.ftp_server,
                                     sync_settings.ftp_port.unwrap_or(21));
        let mut ftp_stream = FtpStream::connect(connection_str.as_str()).unwrap();
        println!("Enter password for ftp server '{}'",
                 sync_settings.ftp_user.as_str());
        let mut passwd = String::new();
        io::stdin().read_line(&mut passwd).expect("Couldn't read password from standard input");
        passwd.pop();  // remove newline
        if passwd.ends_with("\r") {
            passwd.pop();  // remove carriage returns
        }
        ftp_stream.login(sync_settings.ftp_user.as_str(), passwd.as_str())
            .expect("FTP Login failed.");
        ftp_stream
    }

    pub fn execute(&mut self) {
        // create new checksums file
        let hash_file_path = PathBuf::from(self.conf.output_dir.as_ref().unwrap())
            .join(CRC_FILE_NAME);
        let hash_file = hash_file_path.clone()
            .into_os_string()
            .into_string()
            .expect("hash file could not be determined");
        ops::write_hashes(&(hash_file, hash_file_path),
                          Algorithm::CRC64,
                          self.hashsums.clone());

        // retrieve old checksums
        let old_hash_file_path = PathBuf::from(self.conf.output_dir.as_ref().unwrap())
            .join(OLD_CRC_FILE_NAME);
        self.ftp_stream
            .transfer_type(FileType::Binary)
            .expect("FTP couldn't switch to binary mode.");
        match self.ftp_stream
            .retr(CRC_FILE_NAME, |stream| {
                let mut buf = Vec::new();
                let mut f = File::create(old_hash_file_path.as_path())
                    .expect(format!("Couldn't create file '{}'.", OLD_CRC_FILE_NAME).as_str());
                stream.read_to_end(&mut buf)
                    .map(|_| f.write_all(buf.as_slice()).expect("Couldn't write hashsums file."))
                    .map_err(|e| FtpError::ConnectionError(e))
            })
            .and_then(|_| {
                // load old checksums
                let old_hash_file = old_hash_file_path.clone()
                    .into_os_string()
                    .into_string()
                    .expect("old hash file could not be determined");
                ops::read_hashes(&mut stderr(), &(old_hash_file, old_hash_file_path))
                    .map_err(|e| {
                        FtpError::ConnectionError(io::Error::new(io::ErrorKind::Other,
                                                                 format!("old hash file could \
                                                                          not be read: {:?}",
                                                                         e)))
                    })
            })
            .and_then(|old_hashsums| {
                let _ = fs::remove_file(self.output_path.join(OLD_CRC_FILE_NAME).as_path());
                // compare checksums
                ops::compare_hashes(CRC_FILE_NAME, self.hashsums.clone(), old_hashsums)
                    .map_err(|e| {
                        FtpError::ConnectionError(io::Error::new(io::ErrorKind::Other,
                                                                 format!("Couldn't compare hash \
                                                                          files: {:?}",
                                                                         e)))
                    })
            }) {
            Ok((compare_result, compare_file_result)) => {
                for entry in compare_result {
                    match entry {
                        CompareResult::FileRemoved(file) => println!("Removing file {:?}", file),
                        CompareResult::FileAdded(file) => {
                            println!("Adding file {:?}", file.as_str());
                            let path = self.output_path.join(file.as_str());
                            if path.is_dir() {
                                self.create_and_change_to_directory(path.as_path());
                            } else {
                                self.push_file(path.as_path());
                            }
                        }
                        _ => {}
                    }
                }
                for entry in compare_file_result {
                    match entry {
                        CompareFileResult::FileDiffers { file, was_hash: _, new_hash: _ } => {
                            println!("Updateing file {:?}", file);
                            let path = self.output_path.join(file.as_str());
                            if path.is_dir() {
                                self.create_and_change_to_directory(path.as_path());
                            } else {
                                self.push_file(path.as_path());
                            }
                        }
                        _ => {}
                    }
                }
            }

            Err(_) => {
                self.push_all_files();
            }
        };
        let crc_file_path = self.output_path.join(CRC_FILE_NAME);
        let _ = self.push_file(crc_file_path.as_path());
    }

    pub fn push_all_files(&mut self) {
        println!("Push all files to FTP server {}",
                 self.conf.sync_settings.as_ref().unwrap().ftp_server.as_str());
        let walker = WalkDir::new(self.output_path.as_path())
            .min_depth(1)
            .into_iter();
        for entry in walker {
            let entry = entry.unwrap();
            if filter::is_directory(&entry) {
                self.create_and_change_to_directory(entry.path());
            } else {
                self.push_file(entry.path());
            }
        }
    }

    fn create_and_change_to_directory(&mut self, src_dir: &Path) {
        self.ftp_stream.cwd("/").expect("Couldn't change to root dir");
        for comp in src_dir.components().skip(self.output_path_offset) {
            let dir = comp.as_ref()
                .to_str()
                .expect(format!("Couldn't convert component '{:?}' to string.", comp).as_ref());
            if self.ftp_stream.cwd(dir).is_err() {
                println!("Creating '{}'", dir);
                self.ftp_stream
                    .mkdir(dir)
                    .expect(format!("Couldn't create dir '{}'", &dir).as_ref());
                self.ftp_stream
                    .cwd(dir)
                    .expect(format!("Couldn't change to dir '{}'", &dir).as_ref());
            }
        }
    }

    fn push_file(&mut self, src_path: &Path) {
        self.create_and_change_to_directory(src_path.parent().unwrap());
        let file_name = src_path.file_name().and_then(|s| s.to_str()).unwrap();
        File::open(src_path)
            .and_then(|mut f| {
                self.ftp_stream.put(file_name, &mut f).map_err(|e| {
                    io::Error::new(io::ErrorKind::Other,
                                   format!("Error during ftp put operation: {:?}", e).as_ref())
                })
            })
            .expect(format!("Couldn't push file '{}' to ftp server", file_name).as_ref());
    }
}

impl<'a> Drop for Synchronizer<'a> {
    fn drop(&mut self) {
        let _ = self.ftp_stream.quit();
    }
}