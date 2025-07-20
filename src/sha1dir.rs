use anyhow::Result;
use memmap::Mmap;
use rayon::{Scope, ThreadPoolBuilder};
use sha1::{Digest, Sha1};
use std::cmp;
use std::collections::BTreeMap;
use std::env;
use std::fmt::{self, Display};
use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static START_DIR: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| PathBuf::from("."));

pub fn create_hashes(dir: &Path) -> Result<BTreeMap<PathBuf, [u8; 20]>> {
    debug_assert!(dir.is_absolute());
    ThreadPoolBuilder::new()
        .num_threads(cmp::min(std::thread::available_parallelism()?.get(), 8))
        .build_global()?;
    let checksums = build_checksums(dir)?;
    for err in checksums.errors {
        log::error!("{err:#?}");
    }
    Ok(checksums.bytes_map)
}

pub struct Checksums {
    bytes_map: BTreeMap<PathBuf, [u8; 20]>,
    errors: Vec<String>,
}

impl Checksums {
    const fn new() -> Self {
        Self {
            bytes_map: BTreeMap::new(),
            errors: Vec::new(),
        }
    }
}

struct ChecksumsBuilder {
    data: Mutex<Checksums>,
}

impl Display for ChecksumsBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data = self.data.lock().unwrap();
        for entry in &data.bytes_map {
            write!(f, "{} ", entry.0.display())?;
            for &byte in entry.1 {
                write!(f, "{byte:02x}")?;
            }
            writeln!(f).unwrap();
        }
        if !data.errors.is_empty() {
            writeln!(f, "Following errors occurred: ").unwrap();
            for err in &data.errors {
                writeln!(f, "{err:#?}").unwrap();
            }
        }
        Ok(())
    }
}

impl ChecksumsBuilder {
    const fn new() -> Self {
        Self {
            data: Mutex::new(Checksums::new()),
        }
    }
    fn put(&self, path: &Path, hash: Sha1) {
        if path == START_DIR.as_path() {
            return;
        }
        let path = path.to_owned();
        {
            let mut data = self.data.lock().unwrap();
            data.bytes_map
                .entry(path.clone())
                .and_modify(|lhs| {
                    for (lhash, rhash) in lhs.iter_mut().zip(hash.clone().finalize()) {
                        *lhash ^= rhash;
                    }
                })
                .or_insert_with(|| hash.clone().finalize().into());
        }
        if let Some(parent_path) = path.parent() {
            self.put(parent_path, hash);
        }
    }
    fn report_error(&self, error: String) {
        let mut data = self.data.lock().unwrap();
        data.errors.push(error);
    }
    fn into(self) -> Checksums {
        self.data.into_inner().unwrap()
    }
}

fn build_checksums(start_path: &Path) -> Result<Checksums> {
    env::set_current_dir(start_path)?;
    let checksums_builder = ChecksumsBuilder::new();
    rayon::scope(|scope| entry(scope, &checksums_builder, START_DIR.as_path()));
    Ok(checksums_builder.into())
}

fn entry<'scope>(scope: &Scope<'scope>, checksums: &'scope ChecksumsBuilder, path: &Path) {
    let Ok(metadata) = path.symlink_metadata() else {
        checksums.report_error(format!(
            "sha1sum: Could not read file metadata of {}",
            path.display()
        ));
        return;
    };
    let file_type = metadata.file_type();
    let result = if file_type.is_file() {
        file(checksums, path, &metadata)
    } else if file_type.is_symlink() {
        symlink(checksums, path)
    } else if file_type.is_dir() {
        dir(scope, checksums, path)
    } else {
        Err(anyhow::anyhow!(
            "File type of {} not supported",
            path.display()
        ))
    };
    if let Err(error) = result {
        checksums.report_error(format!("sha1sum: {}: {error}", path.display()));
    }
}

fn file(checksums: &ChecksumsBuilder, path: &Path, metadata: &Metadata) -> Result<()> {
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

fn symlink(checksums: &ChecksumsBuilder, path: &Path) -> Result<()> {
    let mut sha = begin(path, b'l');
    sha.update(path.read_link()?.to_string_lossy().as_bytes());
    checksums.put(path, sha);

    Ok(())
}

fn dir<'scope>(
    scope: &Scope<'scope>,
    checksums: &'scope ChecksumsBuilder,
    path: &Path,
) -> Result<()> {
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
    sha.update([kind]);
    #[allow(clippy::cast_possible_truncation)]
    sha.update((path_bytes.len() as u32).to_le_bytes());
    sha.update(path_bytes.as_bytes());
    sha
}
