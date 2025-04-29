use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use weldr::FileRefResolver;
use zip::ZipArchive;

pub struct Resolver {
    root: Vec<u8>,
    root_filename: PathBuf,
}

impl Resolver {
    pub fn new(path: impl AsRef<Path>) -> zip::result::ZipResult<Self> {
        let path = path.as_ref();
        let contents = if path.extension() == Some("io".as_ref()) {
            let f = File::open(path)?;
            let mut zip = ZipArchive::new(f)?;
            let mut ldr_file = zip.by_name("model.ldr")?;
            let mut buf = Vec::with_capacity(ldr_file.size() as usize);

            // skip the byte order mark, if present
            ldr_file.by_ref().take(3).read_to_end(&mut buf)?;
            if buf == "\u{FEFF}".as_bytes() {
                buf.clear();
            }

            ldr_file.read_to_end(&mut buf)?;
            buf
        } else {
            std::fs::read(path)?
        };

        Ok(Self {
            root: contents,
            root_filename: path.file_name().unwrap().into(),
        })
    }
}

fn program_files() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        Some("C:/Program Files".into())
    } else if cfg!(target_os = "macos") {
        Some("/Applications".into())
    } else if cfg!(target_os = "linux") {
        let wine_prefix = std::env::var_os("WINE_PREFIX")?;
        Some(Path::new(&wine_prefix).join("drive_c/Program Files"))
    } else {
        None
    }
}

fn ldraw_base_dirs() -> Vec<PathBuf> {
    // TODO: Allow the user to point to additional paths to allow non-Studio libraries

    let mut v = vec![];

    if let Some(program_files) = program_files() {
        for channel in ["Studio 2.0 EarlyAccess", "Studio 2.0"] {
            let ldraw = program_files.join(channel).join("ldraw");
            v.push(ldraw.clone());
            v.push(ldraw.join("UnOfficial"));
        }
    }

    if let Some(data_local) = dirs::data_local_dir() {
        let studio_data = data_local.join("Stud.io");
        v.push(studio_data.join("CustomParts"));

        if let Ok(iter) = std::fs::read_dir(studio_data.join("NewParts/Updates")) {
            for entry in iter {
                let update_dir = entry.unwrap().path();
                if update_dir.ends_with("__MACOSX") {
                    continue;
                }
                v.push(update_dir);
            }
        }
    }

    v.retain(|p| p.exists());
    v
}

fn ldraw_dirs() -> Vec<PathBuf> {
    // primitive quality order: normal, low, high, very low
    let suffixes = ["parts", "p", "p/8", "p/48", "p/4"];
    ldraw_base_dirs()
        .into_iter()
        .flat_map(|base| suffixes.map(|suffix| base.join(suffix)))
        .filter(|p| p.exists())
        .collect()
}

const LDRAW_DIRS: LazyLock<Vec<PathBuf>> = LazyLock::new(ldraw_dirs);

impl FileRefResolver for Resolver {
    fn resolve<P: AsRef<Path>>(&self, filename: P) -> Result<Vec<u8>, weldr::ResolveError> {
        let filename = filename.as_ref();
        if filename == self.root_filename {
            return Ok(self.root.clone());
        }

        let mut paths = LDRAW_DIRS
            .iter()
            .map(|d| d.join(&filename))
            .collect::<Vec<_>>();

        // Some submodel references specify a detail level, but weldr treats it as a relative path,
        // so this breaks if p/48/foo.dat tries to ask for p/48/bar.dat etc.
        // Just strip the prefix and search everywhere, I guess.
        for prefix in [r"48/", r"8/", r"4/"] {
            if let Some(filename) = filename.to_str().and_then(|s| s.strip_prefix(prefix)) {
                paths.extend(LDRAW_DIRS.iter().map(|d| d.join(filename)));
            }
        }

        for path in paths {
            if !path.exists() {
                continue;
            }

            return std::fs::read(path)
                .map_err(|e| weldr::ResolveError::new(filename.to_string_lossy().into_owned(), e));
        }

        Err(weldr::ResolveError::new_raw(
            filename.to_string_lossy().as_ref(),
        ))
    }
}
