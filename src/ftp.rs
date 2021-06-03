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
        let passwd = rpassword::prompt_password_stdout("Password: ")
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
            .with_context(|| format!("Couldn't push file '{}' to ftp server", file_name))
    }
}

impl Drop for Ftp {
    fn drop(&mut self) {
        let _ = self.stream.quit();
    }
}

pub struct Sftp {
    stream: ssh2::Sftp,
    session: ssh2::Session,
}

impl Sftp {
    pub fn new(server: &str, port: u32, user: &str) -> Result<Self> {
        let connection_str = format!("{}:{}", server, port);
        let tcp = TcpStream::connect(connection_str.as_str())?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;
        let passwd = rpassword::prompt_password_stdout("Password: ")
            .context("Couldn't read password from standard input")?;
        session.userauth_password(user, passwd.as_str())?;
        if session.authenticated() {
            Ok(Sftp {
                stream: session.sftp()?,
                session,
            })
        } else {
            Err(anyhow!("SFTP: connection could not be established!"))
        }
    }
}

impl FtpOperations for Sftp {
    fn get(&mut self, path: &Path, local_path: &Path) -> Result<()> {
        let mut file = self.stream.open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map(|_| {
            let mut f = File::create(local_path)
                .unwrap_or_else(|_| panic!("Couldn't create file '{:?}'.", local_path));
            f.write_all(buf.as_slice()).unwrap_or_else(|_| {
                panic!(
                    "FTP: file '{:?}' could not be written to '{:?}'.",
                    path, local_path
                )
            });
        })?;
        Ok(())
    }

    fn cwd(&mut self, _path: &Path) -> Result<()> {
        todo!()
    }

    fn mkdir(&mut self, _path: &Path) -> Result<()> {
        todo!()
    }

    fn rmdir(&mut self, _path: &Path) -> Result<()> {
        todo!()
    }

    fn del(&mut self, _path: &Path) -> Result<()> {
        todo!()
    }

    fn put(&mut self, _path: &Path, _local_path: &Path) -> Result<()> {
        todo!()
    }
}
