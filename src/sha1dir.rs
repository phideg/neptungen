use anyhow::Result;
use lazy_static::lazy_static;
use memmap::Mmap;
use parking_lot::Mutex;
use rayon::{Scope, ThreadPoolBuilder};
use sha1::{Digest, Sha1};
use std::cmp;
use std::collections::BTreeMap;
use std::env;
use std::fmt::{self, Display};
use std::fs::{File, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Once;

lazy_static! {
    static ref START_DIR: PathBuf = PathBuf::from(".");
}

fn die<P: AsRef<Path>, E: Display>(path: P, error: E) -> ! {
    static DIE: Once = Once::new();

    DIE.call_once(|| {
        let path = path.as_ref().display();
        let _ = writeln!(io::stderr(), "sha1sum: {}: {}", path, error);
        process::exit(1);
    });

    unreachable!()
}

pub(crate) fn create_hashes(dir: &Path) -> BTreeMap<PathBuf, [u8; 20]> {
    debug_assert!(dir.is_absolute());
    init_thread_pool();
    let checksums = build_checksums(dir);
    checksums.into()
}

fn init_thread_pool() {
    //TODO: handle error!
    ThreadPoolBuilder::new()
        .num_threads(cmp::min(num_cpus::get(), 8))
        .build_global()
        .unwrap();
}

struct Checksums {
    bytes_map: Mutex<BTreeMap<PathBuf, [u8; 20]>>,
}

impl Checksums {
    fn new() -> Self {
        Checksums {
            bytes_map: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Display for Checksums {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let map = self.bytes_map.lock();
        for entry in map.iter() {
            write!(f, "{:?} ", entry.0)?;
            for &byte in entry.1 {
                write!(f, "{:02x}", byte)?;
            }
        }
        Ok(())
    }
}

impl Checksums {
    fn put(&self, path: &Path, hash: Sha1) {
        if path == START_DIR.as_path() {
            return;
        }
        let path = path.to_owned();
        {
            let mut map = self.bytes_map.lock();
            map.entry(path.clone())
                .and_modify(|lhs| {
                    for (lhash, rhash) in lhs.iter_mut().zip(hash.clone().finalize()) {
                        *lhash ^= rhash;
                    }
                })
                .or_insert_with(|| hash.clone().finalize().into());
        }
        if let Some(parent_path) = path.parent() {
            // TODO: handle errors
            self.put(parent_path, hash);
        }
    }

    fn into(self) -> BTreeMap<PathBuf, [u8; 20]> {
        self.bytes_map.into_inner()
    }
}

fn build_checksums(start_path: &Path) -> Checksums {
    // TODO: error handling
    env::set_current_dir(start_path).unwrap();
    let checksums = Checksums::new();
    rayon::scope(|scope| entry(scope, &checksums, START_DIR.as_path()));
    checksums
}

fn entry<'scope>(scope: &Scope<'scope>, checksums: &'scope Checksums, path: &Path) {
    let metadata = match path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) => die(path, error),
    };

    let file_type = metadata.file_type();
    let result = if file_type.is_file() {
        file(checksums, path, metadata)
    } else if file_type.is_symlink() {
        symlink(checksums, path)
    } else if file_type.is_dir() {
        dir(scope, checksums, path)
    } else {
        die(path, "Unsupported file type");
    };

    if let Err(error) = result {
        die(path, error);
    }
}

fn file(checksums: &Checksums, path: &Path, metadata: Metadata) -> Result<()> {
    let mut sha = begin(path, b'f');

    // Enforced by memmap: "memory map must have a non-zero length"
    if metadata.len() > 0 {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        sha.update(&mmap);
    }
    checksums.put(path, sha);

    Ok(())
}

fn symlink(checksums: &Checksums, path: &Path) -> Result<()> {
    let mut sha = begin(path, b'l');
    sha.update(path.read_link()?.to_string_lossy().as_bytes());
    checksums.put(path, sha);

    Ok(())
}

fn dir<'scope>(scope: &Scope<'scope>, checksums: &'scope Checksums, path: &Path) -> Result<()> {
    let sha = begin(path, b'd');
    checksums.put(path, sha);

    for child in path.read_dir()? {
        let child = child?.path();
        scope.spawn(move |scope| entry(scope, checksums, &child));
    }

    Ok(())
}

fn begin(path: &Path, kind: u8) -> Sha1 {
    let mut sha = Sha1::new();
    let path_bytes = path.to_string_lossy();
    sha.update(&[kind]);
    sha.update(&(path_bytes.len() as u32).to_le_bytes());
    sha.update(path_bytes.as_bytes());
    sha
}
