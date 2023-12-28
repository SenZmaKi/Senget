//!Global variables and utility structs, enums and functions

use reqwest::{header, Client};
use spinners::{Spinner, Spinners};
use std::{
    env, io,
    path::{Path, PathBuf},
};

pub const EXPORTED_PACKAGES_FILENAME: &str = "senget-packages.txt";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Github package manager for windows";
pub const MSI_EXEC: &str = "MsiExec.exe";
pub const IBYTES_TO_MBS_DIVISOR: u64 = 1024 * 1024;
// TODO: set to false on deployment
pub const DEBUG: bool = true;

pub fn root_dir() -> PathBuf {
    if DEBUG {
        return PathBuf::from(".");
    };
    env::current_exe().unwrap().parent().unwrap().to_owned()
}

pub fn display_path(path: &Path) -> Result<String, io::Error> {
    // Remove the weird canonicalised path delimeter e.g.,
    // \\?\C:\Users\PC\OneDrive -> C:\Users\PC\OneDrive
    Ok(path
        .canonicalize()?
        .display()
        .to_string()
        .replace("\\\\?\\", ""))
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
    use crate::includes::error::PrivilegeError;
    use crate::includes::utils::loading_animation;
    use std::thread;
    use std::time::Duration;
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
}
