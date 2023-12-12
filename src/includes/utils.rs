//!Global variables and utility structs, enums and functions

use reqwest::{header, Client};
use spinners::{Spinner, Spinners};
use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};


pub const APP_NAME: &str = "Senget";
pub const APP_NAME_LOWER: &str = "senget";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str = "Github package manager";
// TODO set to false on deployment
pub const DEBUG: bool = true;


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
            let load_anim = loading_animation();
            let join_handle = load_anim.start("Fondling balls.. .".to_owned());
            thread::sleep(Duration::new(5, 0));
            load_anim.stop(join_handle);
        };
        run();
        thread::sleep(Duration::new(2, 0));
        run();
    }
}
