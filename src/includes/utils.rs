//!Global variables and utility structs, enums and functions

use reqwest::{header, Client};
use spinners::{Spinner, Spinners};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle}, path::PathBuf, io,
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
    Ok(path.canonicalize()?.display().to_string().replace("\\\\?\\", ""))
}
pub struct LoadingAnimation {
    stop_flag: Arc<(Mutex<Option<bool>>, Condvar)>,
    join_handle: Option<JoinHandle<()>>,
}

impl LoadingAnimation {
    pub fn new() -> LoadingAnimation {
        let stop_flag = Arc::new((Mutex::new(None), Condvar::new()));
        LoadingAnimation {
            stop_flag,
            join_handle: None,
        }
    }

    pub fn start(&mut self, task: String) {
        let continue_flag = self.stop_flag.clone();
        let handle = thread::spawn(move || {
            let mut spinner = Spinner::new(Spinners::Material, task);
            let (lock, cvar) = &*continue_flag;
            let mut guard = lock.lock().unwrap();
            while guard.is_none() {
                guard = cvar.wait(guard).unwrap();
            }
            if guard.unwrap() {
                spinner.stop_and_persist("✔", "Finished\n".to_owned());
            } else {
                spinner.stop_and_persist("✘", "Failed\n".to_owned());
            }
        });

        self.join_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        *self.stop_flag.0.lock().unwrap() = Some(true);
        self.stop_flag.1.notify_one();
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().unwrap();
        }
    }
}

impl Drop for LoadingAnimation {
    fn drop(&mut self) {
        *self.stop_flag.0.lock().unwrap() = Some(false);
        self.stop_flag.1.notify_one();
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().unwrap();
        }
    }
}


// pub fn handle_network_error()

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
    use crate::includes::test_utils::loading_animation;
    use std::thread;
    use std::time::Duration;
    #[test]
    fn test_loading() {
        let run = || {
            let mut load_anim = loading_animation();
            load_anim.start("Fondling balls.. .".to_owned());
            thread::sleep(Duration::new(5, 0));
            load_anim.stop();
        };
        run();
        thread::sleep(Duration::new(2, 0));
        run();
    }
}
