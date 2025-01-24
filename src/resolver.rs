use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
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

impl FileRefResolver for Resolver {
    fn resolve<P: AsRef<Path>>(&self, filename: P) -> Result<Vec<u8>, weldr::ResolveError> {
        let filename = filename.as_ref();
        if filename == self.root_filename {
            return Ok(self.root.clone());
        }

        for dir in [
            "c:/Users/Andrew/AppData/Local/Stud.io/CustomParts/parts",
            "c:/Program Files/Studio 2.0/ldraw/p/48",
            "c:/Program Files/Studio 2.0/ldraw/p",
            "c:/Program Files/Studio 2.0/ldraw/p/8",
            "c:/Program Files/Studio 2.0/ldraw/p/4",
            "c:/Program Files/Studio 2.0/ldraw/parts",
            "c:/Program Files/Studio 2.0/ldraw/UnOfficial/parts",
            "c:/Program Files/Studio 2.0/ldraw/UnOfficial/p",
        ] {
            let path = Path::new(dir).join(&filename);
            if path.exists() {
                return std::fs::read(path).map_err(|e| {
                    weldr::ResolveError::new(filename.to_string_lossy().into_owned(), e)
                });
            }
        }
        Err(weldr::ResolveError::new_raw(
            filename.to_string_lossy().as_ref(),
        ))
    }
}
