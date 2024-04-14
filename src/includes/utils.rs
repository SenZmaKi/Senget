//!Global variables and utility traits, structs, enums and functions

use reqwest::{header, Client};
use spinners::{Spinner, Spinners};
use std::{
    env,
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
    process::Command,
};

#[macro_export]
macro_rules! success_println_pretty {
    ($($arg:tt)*) => {
        println!("\x1b[32m{}\x1b[0m", format!($($arg)*))
    };
}

#[macro_export]
macro_rules! eprintln_pretty {
    ($($arg:tt)*) => {
        eprintln!("\x1b[31m{}\x1b[0m", format!($($arg)*))
    };
}

pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");
pub const IBYTES_TO_MBS_DIVISOR: u64 = 1024 * 1024;
pub const DEBUG: bool = cfg!(debug_assertions);
pub const EXPORTED_PACKAGES_FILENAME: &str = "senget-packages.json";

pub trait Cmd {
    fn cmd() -> Command;
}
impl Cmd for Command {
    fn cmd() -> Command {
        let mut command = Command::new("cmd");
        command.arg("/c");
        command
    }
}

pub trait Take<T> {
    fn take(self, index: usize) -> Option<T>;
}

impl<T> Take<T> for Vec<T> {
    fn take(self, index: usize) -> Option<T> {
        self.into_iter().nth(index)
    }
}
pub trait MoveDirAll {
    fn move_dir_all(&self, to: &Path) -> Result<(), io::Error>;
}

impl MoveDirAll for Path {
    fn move_dir_all(&self, to: &Path) -> Result<(), io::Error> {
        fn helper(from: &Path, to: &Path) -> Result<(), io::Error> {
            let folder_items = from.folder_items()?;
            if to.is_file() {
                fs::remove_file(to)?;
            }
            if !to.is_dir() {
                fs::create_dir(to)?;
            }
            for item in folder_items {
                let item_to = &to.join(item.file_name());
                let item_path = item.path();
                if item_path.is_dir() {
                    helper(&item_path, item_to)?;
                } else {
                    if item_to.is_file() {
                        dbg!(item_to);
                        fs::remove_file(item_to)?;
                    } else if item_to.is_dir() {
                        fs::remove_dir_all(item_to)?;
                    }
                    fs::rename(item_path, item_to)?;
                }
            }
            fs::remove_dir(from)
        }
        helper(self, to)
    }
}

pub trait FolderItems {
    fn folder_items(&self) -> Result<Vec<DirEntry>, io::Error>;
}

impl FolderItems for Path {
    fn folder_items(&self) -> Result<Vec<DirEntry>, io::Error> {
        self.read_dir()?.try_fold(
            Vec::new(),
            |mut vec, de| -> Result<Vec<DirEntry>, io::Error> {
                vec.push(de?);
                Ok(vec)
            },
        )
    }
}

pub trait FilenameLower {
    fn filename_lower(&self) -> String;
}
impl FilenameLower for Path {
    fn filename_lower(&self) -> String {
        self.file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_lowercase()
    }
}

pub trait PathStr {
    fn path_str(&self) -> Result<String, io::Error>;
}

impl PathStr for Path {
    fn path_str(&self) -> Result<String, io::Error> {
        // Remove the weird canonicalised path delimeter e.g.,
        // \\?\C:\Users\PC\OneDrive -> C:\Users\PC\OneDrive
        let ps = self
            .canonicalize()?
            .display()
            .to_string()
            .replace("\\\\?\\", "");
        Ok(ps)
    }
}

pub fn root_dir() -> PathBuf {
    if DEBUG {
        return PathBuf::from(".");
    };
    env::current_exe().unwrap().parent().unwrap().to_owned()
}

pub fn loading_animation<T, E, F>(task_title: String, task: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    let mut spinner = Spinner::new(Spinners::Dots, task_title);
    match task() {
        Ok(ok) => {
            spinner.stop_and_persist("✔", "\x1b[32mFinished\x1b[0m".to_owned());
            Ok(ok)
        }
        Err(err) => {
            spinner.stop_and_persist("\x1b[31m✘\x1b[0m", "\x1b[31mFailed\x1b[0m".to_owned());
            Err(err)
        }
    }
}

pub fn setup_client() -> Result<Client, reqwest::Error> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Senget"),
    );
    Client::builder().default_headers(headers).build()
}

