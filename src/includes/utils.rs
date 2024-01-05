//!Global variables and utility structs, enums and functions

use reqwest::{header, Client};
use spinners::{Spinner, Spinners};
use std::{
    env,
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

pub const EXPORTED_PACKAGES_FILENAME: &str = "senget-packages.json";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Github package manager for windows";
pub const MSI_EXEC: &str = "MsiExec.exe";
pub const IBYTES_TO_MBS_DIVISOR: u64 = 1024 * 1024;
// FIXME: set to false on deployment
pub const DEBUG: bool = true;
pub trait MoveDirAll {
    fn move_dir_all(&self, to: &Path) -> Result<(), io::Error>;
}

impl MoveDirAll for Path {
    fn move_dir_all(&self, to: &Path) -> Result<(), io::Error> {
        move_dir_all(self, to)
    }
}

pub trait FolderItems {
    fn fetch_folder_items(&self) -> Result<Vec<DirEntry>, io::Error>;
}

impl FolderItems for Path {
    fn fetch_folder_items(&self) -> Result<Vec<DirEntry>, io::Error> {
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

fn move_dir_all(from: &Path, to: &Path) -> Result<(), io::Error> {
    let folder_items = from.fetch_folder_items()?;
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
            move_dir_all(&item_path, item_to)?;
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
    let mut spinner = Spinner::new(Spinners::Material, task_title);
    match task() {
        Ok(ok) => {
            spinner.stop_and_persist("✔", "Finished".to_owned());
            Ok(ok)
        }
        Err(err) => {
            spinner.stop_and_persist("✘", "Failed".to_owned());
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

#[cfg(test)]
mod tests {
    use crate::includes::utils::loading_animation;
    use crate::includes::{error::PrivilegeError, utils::FolderItems};
    use std::thread;
    use std::time::Duration;

    use super::{root_dir, MoveDirAll};
    fn actual_task(fail: bool) -> Result<String, PrivilegeError> {
        thread::sleep(Duration::new(5, 0));
        if fail {
            Err(PrivilegeError)
        } else {
            Ok("Fondled".to_owned())
        }
    }
    #[test]
    fn test_loading() {
        let result = loading_animation("Fondling Balls".to_owned(), || actual_task(false));
        assert!(result.is_ok());
        let result = loading_animation("Fondling Balls".to_owned(), || actual_task(true));
        assert!(result.is_err());
    }

    #[test]
    fn test_move_dir_all() {
        let test_move_dir = root_dir().join("test-move-dir-all");
        let from = test_move_dir.join("from");
        let to = test_move_dir.join("to");
        from.move_dir_all(&to).expect("Folder was moved");
        assert!(!to.fetch_folder_items().unwrap().is_empty());
    }
}

