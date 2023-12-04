//!Global variables and utility classes/functions

use lazy_static::lazy_static;
use reqwest::{header, Client};

pub const APP_NAME: &str = "Senget";

pub fn fatal_error(err: &(dyn std::error::Error + 'static)) -> ! {
    panic!("Fatal Error: {}", err);
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
    input.chars().filter(|c| c.is_alphabetic()).collect::<String>().to_lowercase()
}

pub fn fuzzy_compare(main: &str, comp: &str) -> bool {
    strip_string(main).contains(comp)
}

lazy_static! {
    pub static ref CLIENT: Client = setup_client();
}
