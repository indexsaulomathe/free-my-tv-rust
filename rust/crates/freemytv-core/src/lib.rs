//! Free My TV's build tools, ported from Python to Rust one component at a time.
//!
//! | Module | Replaces |
//! |---|---|
//! | [`config`] | `read_config`, `GENERIC` in `tools/webapp.py` |
//! | [`i18n`] | `web_keys` in `tools/translate.py`, `catalogues` in `tools/webapp.py` |
//! | [`webapp`] | `files`, `coins`, `preview` in `tools/webapp.py` |
//! | [`package`] | `webos/build.py` (.ipk) and `tizen/build.py` (.wgt, unsigned) |
//! | [`homebrew`] | `tools/homebrew_repo.py` |
//! | [`zones`] | `tools/make_zones.py` |
//! | [`json`] | Python's `json.dumps`, so every file comes out byte for byte the same |

pub mod config;
pub mod homebrew;
pub mod i18n;
pub mod json;
pub mod package;
pub mod webapp;
pub mod zones;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// An error with a message for the person running the tool.
#[derive(Debug, Clone, PartialEq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error(e.to_string())
    }
}

/// A directory's entry names, sorted as Python's `sorted(os.listdir(...))` sorts them.
pub fn sorted_names(dir: &Path) -> Result<Vec<String>, Error> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map_err(|e| Error(format!("{}: {e}", dir.display())))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

/// The repository: where the web app, the assets and the manifests are.
#[derive(Clone, Debug)]
pub struct Repo {
    pub root: PathBuf,
}

impl Repo {
    pub fn new(root: impl Into<PathBuf>) -> Repo {
        Repo { root: root.into() }
    }

    /// The repository holding `start` (or one of its parents): the folder with `webos/appinfo.json`.
    pub fn find(start: &Path) -> Option<Repo> {
        let mut dir = Some(start);
        while let Some(d) = dir {
            if d.join("webos").join("appinfo.json").is_file() && d.join("web").is_dir() {
                return Some(Repo::new(d));
            }
            dir = d.parent();
        }
        None
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn read(&self, rel: &str) -> Result<Vec<u8>, Error> {
        fs::read(self.path(rel)).map_err(|e| Error(format!("{rel}: {e}")))
    }

    pub fn read_string(&self, rel: &str) -> Result<String, Error> {
        fs::read_to_string(self.path(rel)).map_err(|e| Error(format!("{rel}: {e}")))
    }
}
