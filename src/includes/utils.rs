//!Global variables and utility structs, enums and functions

use crate::includes::github::api::Repo;
use lazy_static::lazy_static;
use reqwest::{header, Client, Request};
use spinners::{Spinner, Spinners};
use std::{
    io,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

use super::{install::InstallInfo, package::Package};

pub const APP_NAME: &str = "Senget";
pub type GenericError = Box<dyn std::error::Error>;

lazy_static! {
    pub static ref PACKAGE_INSTALLER_DIR: PathBuf = PathBuf::from("Package-Installers");
    // Test utils
    pub static ref SENPWAI_REPO: Repo = Repo::new(
        "Senpwai".to_owned(),
        "SenZmaKi/Senpwai".to_owned(),
        "https://github.com/senzmaki/senpwai".to_owned(),
        Some("A desktop app for batch downloading anime".to_owned()),
        Some("Python".to_owned()),
    );
    pub static ref SENPWAI_PACKAGE: Package = make_senpwai_package();
    pub static ref LOADING_ANIMATION: LoadingAnimation = LoadingAnimation::new();
}
fn make_senpwai_package() -> Package {
    let install_info = InstallInfo {
        executable_path: Some(PathBuf::from(
            "C:\\Users\\PC\\AppData\\Local\\Programs\\Senpwai\\Senpwai.exe",
        )),
        uninstall_command: Some(
            "C:\\Users\\PC\\AppData\\Local\\Programs\\Senpwai\\unins000.exe /SILENT".to_owned(),
        ),
    };
    Package::new("2.0.6".to_owned(), (*SENPWAI_REPO).to_owned(), install_info)
}

pub fn fatal_error(err: &(dyn std::error::Error + 'static)) -> ! {
    panic!("Fatal Error: {}", err);
}
pub struct LoadingAnimation {
    stop_flag: Arc<(Mutex<bool>, Condvar)>,
}

impl LoadingAnimation {
    pub fn new() -> LoadingAnimation {
        let stop_flag = Arc::new((Mutex::new(false), Condvar::new()));
        LoadingAnimation { stop_flag }
    }

    pub fn start(&self, task: String) -> JoinHandle<()> {
        let continue_flag = self.stop_flag.clone();
        thread::spawn(move || {
            let mut spinner = Spinner::new(Spinners::Material, task);
            let (lock, cvar) = &*continue_flag;
            let mut guard = lock.lock().unwrap();
            // To avoid spurious wakeups we must check if the value of guard bool
            // really changed when the thread was woke up or it was just a spurious wakeup
            while !*guard {
                guard = cvar.wait(guard).unwrap();
            }
            spinner.stop_and_persist("✔", "Finished\n".to_owned());
        })
    }

    pub fn stop(&self, join_handle: JoinHandle<()>) {
        *self.stop_flag.0.lock().unwrap() = true;
        self.stop_flag.1.notify_one();
        join_handle.join().unwrap();
        *self.stop_flag.0.lock().unwrap() = false;
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

pub fn strip_string(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase()
}
use std::fmt;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}
impl Error {
    pub fn new(message: String) -> Error {
        Error { message }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}Error: {}", APP_NAME, self.message)
    }
}
impl std::error::Error for Error {}
pub fn fuzzy_compare(main: &str, comp: &str) -> bool {
    strip_string(main).contains(comp)
}

#[cfg(test)]
mod tests {
    use crate::utils::LOADING_ANIMATION;
    use std::thread;
    use std::time::Duration;
    #[test]
    fn test_loading() {
        let run = || {
            let join_handle = LOADING_ANIMATION.start("Fondling balls.. .".to_owned());
            thread::sleep(Duration::new(5, 0));
            LOADING_ANIMATION.stop(join_handle);
        };
        run();
        thread::sleep(Duration::new(2, 0));
        run();
    }
}
