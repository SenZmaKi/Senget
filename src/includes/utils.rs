//!Global variables and utility structs/enums/functions

use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
};

use lazy_static::lazy_static;
use reqwest::{header, Client};
use spinners::{Spinner, Spinners};

pub const APP_NAME: &str = "Senget";

lazy_static! {
    pub static ref CLIENT: Client = setup_client();
}
pub fn fatal_error(err: &(dyn std::error::Error + 'static)) -> ! {
    panic!("Fatal Error: {}", err);
}
pub struct LoadingAnimation {
    stop_flag: Arc<(Mutex<bool>, Condvar)>,
    task: String,
    join_handle: Option<JoinHandle<()>>,
}

impl LoadingAnimation {
    pub fn new(task: &str) -> LoadingAnimation {
        let stop_flag = Arc::new((Mutex::new(false), Condvar::new()));
        let task = task.to_owned();
        LoadingAnimation {
            stop_flag,
            task,
            join_handle: None,
        }
    }

    pub fn start(&mut self) {
        let task = self.task.clone();
        let continue_flag = self.stop_flag.clone();
        let jh = thread::spawn(move || {
            let mut spinner = Spinner::new(Spinners::Material, task);
            let (lock, cvar) = &*continue_flag;
            let mut guard = lock.lock().unwrap();
            // To avoid spurious wakeups we must check if the value of guard bool
            // really changed when the thread was woke up or it was just a spurious wakeup
            while !*guard {
                guard = cvar.wait(guard).unwrap();
            }
            spinner.stop();
        });
        self.join_handle = Some(jh);
    }

    pub fn stop(&mut self) {
        if let Some(jh) = self.join_handle.take() {
            *self.stop_flag.0.lock().unwrap() = true;
            self.stop_flag.1.notify_one();
            jh.join().unwrap();
            *self.stop_flag.0.lock().unwrap() = false;
        }
    }
}

pub fn setup_client() -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(APP_NAME),
    );
    return Client::builder().default_headers(headers).build().unwrap();
}

pub fn strip_string(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
        .to_lowercase()
}

pub fn fuzzy_compare(main: &str, comp: &str) -> bool {
    strip_string(main).contains(comp)
}

#[cfg(test)]
mod tests {
    use super::{thread, LoadingAnimation};
    use std::time::Duration;
    #[test]
    fn test_loading() {
        let mut load = LoadingAnimation::new("Fondling balls.. .");
        let mut run = || {
            load.start();
            thread::sleep(Duration::new(5, 0));
            load.stop();
        };
        run();
        thread::sleep(Duration::new(2, 0));
        run();
    }
}
