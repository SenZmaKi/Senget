//!Global variables and functions

use reqwest::{Client, header};
use once_cell::sync::Lazy;

pub const APP_NAME: &str = "Senget";

pub fn setup_client() -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static(APP_NAME));
    return Client::builder().default_headers(headers).build().unwrap();
}   

pub static CLIENT: Lazy<Client> = Lazy::new(setup_client);
