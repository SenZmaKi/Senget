//!Global variables and utility structs, enums and functions

use reqwest::{header, Client};
use spinners::{Spinner, Spinners};
use std::{
    io,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

pub const APP_NAME: &str = "Senget";
pub const APP_NAME_LOWER: &str = "senget";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str = "Github package manager";
pub const MSI_EXEC: &str = "MsiExec.exe";
// TODO set to false on deployment
pub const DEBUG: bool = true;

pub fn display_path(path: &PathBuf) -> Result<String, io::Error> {
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
        header::HeaderValue::from_static(APP_NAME),
    );
    return Ok(Client::builder().default_headers(headers).build()?);
}

#[cfg(test)]
mod tests {
    use crate::includes::error::{KnownErrors, PrivilegeError};
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
