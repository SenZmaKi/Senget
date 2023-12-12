//! Contains error handling utility

use reqwest;
use std::fmt;
use std::io;
use tinydb;

pub struct ContentLengthError;

impl fmt::Debug for ContentLengthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ContentLengthError: Invalid content length")
    }
}
pub struct PrivilegeError;
impl fmt::Debug for PrivilegeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PrivilegeError: Rerun the command in an admin shell, e.g., if you're using Command Prompt, run it as an Administrator"
        )
    }
}

#[derive(Debug)]
pub enum KnownErrors {
    RequestError(reqwest::Error),
    IoError(io::Error),
    DatabaseError(tinydb::error::DatabaseError),
    PrivilegeError(PrivilegeError),
    RequestIoError(RequestIoError),
    RequestIoContentLengthError(RequestIoContentLengthError),
}

#[derive(Debug)]
pub enum RequestIoContentLengthError {
    RequestIoError(RequestIoError),
    ContentLengthError(ContentLengthError),
}

#[derive(Debug)]
pub enum RequestIoError {
    IoError(io::Error),
    RequestError(reqwest::Error),
}

impl From<io::Error> for RequestIoError {
    fn from(error: io::Error) -> Self {
        RequestIoError::IoError(error)
    }
}

impl From<reqwest::Error> for RequestIoError {
    fn from(error: reqwest::Error) -> Self {
        RequestIoError::RequestError(error)
    }
}

impl From<ContentLengthError> for RequestIoContentLengthError {
    fn from(error: ContentLengthError) -> Self {
        RequestIoContentLengthError::ContentLengthError(error)
    }
}

impl From<io::Error> for RequestIoContentLengthError {
    fn from(error: io::Error) -> Self {
        RequestIoContentLengthError::RequestIoError(RequestIoError::IoError(error))
    }
}

impl From<reqwest::Error> for RequestIoContentLengthError {
    fn from(error: reqwest::Error) -> Self {
        RequestIoContentLengthError::RequestIoError(RequestIoError::RequestError(error))
    }
}

impl From<reqwest::Error> for KnownErrors {
    fn from(error: reqwest::Error) -> Self {
        KnownErrors::RequestError(error)
    }
}

impl From<PrivilegeError> for KnownErrors {
    fn from(error: PrivilegeError) -> Self {
        KnownErrors::PrivilegeError(error)
    }
}
impl From<io::Error> for KnownErrors {
    fn from(error: io::Error) -> Self {
        KnownErrors::IoError(error)
    }
}
impl From<tinydb::error::DatabaseError> for KnownErrors {
    fn from(error: tinydb::error::DatabaseError) -> Self {
        KnownErrors::DatabaseError(error)
    }
}

impl From<RequestIoError> for KnownErrors {
    fn from(error: RequestIoError) -> Self {
        KnownErrors::RequestIoError(error)
    }
}

impl From<RequestIoContentLengthError> for KnownErrors {
    fn from(error: RequestIoContentLengthError) -> Self {
        KnownErrors::RequestIoContentLengthError(error)
    }
}
fn handle_privilege_error(err: KnownErrors) -> KnownErrors {
    let str_error = format!("{:?}", err);
    if str_error.contains("The requested operation requires elevation.") {
        return PrivilegeError.into();
    };
    err
}

pub fn print_error(err: KnownErrors) {
    let err = handle_privilege_error(err);
    eprintln!("\n{:?}", err);
}
