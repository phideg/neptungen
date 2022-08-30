use crate::last_path_comp_as_str;
use anyhow::{anyhow, Context, Result};
use ftp::types::FileType;
use ftp::FtpError;
use ftp::FtpStream;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Component, Path};

pub trait FtpOperations {
    fn get(&mut self, path: &Path, local_path: &Path) -> Result<()>;
    fn put(&mut self, path: &Path, local_path: &Path) -> Result<()>;
    fn del(&mut self, path: &Path) -> Result<()>;
    fn cwd(&mut self, path: &Path) -> Result<()>;
    fn rmdir(&mut self, path: &Path) -> Result<()>;
    fn mkdir(&mut self, path: &Path) -> Result<()>;
}

pub struct Ftp {
    stream: FtpStream,
}

impl Ftp {
    pub fn new(server: &str, port: u32, user: &str) -> Result<Self> {
        let connection_str = format!("{}:{}", server, port);
        let mut ftp = Ftp {
            stream: FtpStream::connect(connection_str.as_str())?,
        };
        println!("Enter password for ftp server '{}' user '{}'", server, user);
        let passwd = rpassword::prompt_password("Password: ")
            .context("Couldn't read password from standard input")?;
        ftp.stream
            .login(user, passwd.as_str())
            .context("FTP: Login failed.")?;
        ftp.stream
            .transfer_type(FileType::Binary)
            .context("FTP: couldn't switch to binary mode.")?;
        Ok(ftp)
    }
}

impl FtpOperations for Ftp {
    fn get(&mut self, path: &Path, local_path: &Path) -> Result<()> {
        if let Some(Component::Normal(remote_file)) = path.components().last() {
            let remote_file = remote_file.to_str().context("Invalid path component")?;
            self.stream.retr(remote_file, |stream| {
                let mut buf = Vec::new();
                stream
                    .read_to_end(&mut buf)
                    .map(|_| {
                        let mut f = File::create(local_path)
                            .unwrap_or_else(|_| panic!("Couldn't create file '{:?}'.", local_path));
                        f.write_all(buf.as_slice()).unwrap_or_else(|_| {
                            panic!(
                                "FTP: file '{}' could not be written to '{:?}'.",
                                remote_file, local_path
                            )
                        });
                    })
                    .map_err(FtpError::ConnectionError)
            })?;
        }
        Ok(())
    }

    fn cwd(&mut self, path: &Path) -> Result<()> {
        self.stream
            .cwd(last_path_comp_as_str!(path)?)
            .with_context(|| format!("FTP: Couldn't change to dir '{:?}'", path))
    }

    fn mkdir(&mut self, path: &Path) -> Result<()> {
        self.stream
            .mkdir(last_path_comp_as_str!(path)?)
            .with_context(|| format!("FTP: Couldn't create dir '{:?}'", path))
    }

    fn rmdir(&mut self, path: &Path) -> Result<()> {
        self.stream
            .rmdir(last_path_comp_as_str!(path)?)
            .with_context(|| format!("FTP: Couldn't remove dir '{:?}'", path))
    }

    fn del(&mut self, path: &Path) -> Result<()> {
        self.stream
            .rm(last_path_comp_as_str!(path)?)
            .with_context(|| format!("FTP: Couldn't remove file '{:?}'", path))
    }

    fn put(&mut self, path: &Path, local_path: &Path) -> Result<()> {
        let mut f = File::open(local_path)?;
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Couldn't determine filename")?;
        self.stream
            .put(file_name, &mut f)
            .with_context(|| format!("FTP: Couldn't push file '{}' to server", file_name))
    }
}

impl Drop for Ftp {
    fn drop(&mut self) {
        let _ = self.stream.quit();
    }
}

pub struct Sftp {
    session: ssh2::Session,
}

impl Sftp {
    pub fn new(server: &str, port: u32, user: &str) -> Result<Self> {
        let connection_str = format!("{}:{}", server, port);
        let tcp = TcpStream::connect(connection_str.as_str())?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        let passwd = rpassword::prompt_password("Password: ")
            .context("Couldn't read password from standard input")?;
        session.userauth_password(user, passwd.as_str())?;
        if session.authenticated() {
            Ok(Sftp { session })
        } else {
            Err(anyhow!("SFTP: connection could not be established!"))
        }
    }
}

impl FtpOperations for Sftp {
    fn get(&mut self, path: &Path, local_path: &Path) -> Result<()> {
        let sftp = self.session.sftp()?;
        let mut file = sftp.open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map(|_| {
            let mut f = File::create(local_path)
                .unwrap_or_else(|_| panic!("Couldn't create file '{:?}'.", local_path));
            f.write_all(buf.as_slice()).unwrap_or_else(|_| {
                panic!(
                    "SFTP: file '{:?}' could not be written to '{:?}'.",
                    path, local_path
                )
            });
        })?;
        Ok(())
    }

    fn cwd(&mut self, path: &Path) -> Result<()> {
        // in contrast to classic ftp this sftp api supports std::Path
        // so we only need to check that the directory exists a cwd operation
        let sftp = self.session.sftp()?;
        sftp.opendir(path)
            .with_context(|| format!("SFTP: couldn't read '{:?}'", path))
            .map(|_| ())
    }

    fn mkdir(&mut self, path: &Path) -> Result<()> {
        // unfortunately there are not constants for the
        // creation mode libssh2 docs advice 0775 as default
        // https://www.libssh2.org/libssh2_sftp_mkdir_ex.html
        let sftp = self.session.sftp()?;
        sftp.mkdir(path, 0o755)
            .with_context(|| format!("SFTP: couldn't create '{:?}'", path))
    }

    fn rmdir(&mut self, path: &Path) -> Result<()> {
        let sftp = self.session.sftp()?;
        sftp.rmdir(path)
            .with_context(|| format!("SFTP: couldn't remove dir '{:?}'", path))
    }

    fn del(&mut self, path: &Path) -> Result<()> {
        let sftp = self.session.sftp()?;
        sftp.unlink(path)
            .with_context(|| format!("SFTP: couldn't remove file '{:?}'", path))
    }

    fn put(&mut self, path: &Path, local_path: &Path) -> Result<()> {
        let sftp = self.session.sftp()?;
        let mut f = File::open(local_path)?;
        let mut remote_file = sftp
            .create(path)
            .with_context(|| format!("SFTP: Couldn't push file '{:?}' to server", path))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map(|_| {
            remote_file.write_all(buf.as_slice()).unwrap_or_else(|_| {
                panic!(
                    "SFTP: file '{:?}' could not be written to '{:?}'.",
                    path, local_path
                )
            });
        })?;
        Ok(())
    }
}
