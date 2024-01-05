//! Contains error handling utility

use reqwest;
use std::fmt;
use std::io;
use tinydb;

pub struct ContentLengthError;

impl fmt::Debug for ContentLengthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ContentLength: Invalid content length")
    }
}
pub struct ExportFileNotFoundError;

impl fmt::Debug for ExportFileNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Export file not found")
    }
}

pub struct NoExeFoundError;
impl fmt::Debug for NoExeFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No executable found for the unpacked zip file")
    }
}
pub struct PrivilegeError;
impl fmt::Debug for PrivilegeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rerun the command in an admin shell, e.g., if you're using Command Prompt, run it as an Administrator."
        )
    }
}

pub struct NetworkError;
impl fmt::Debug for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Check your internet connection and try again.")
    }
}

pub struct NoInstalledPackageError;
impl fmt::Debug for NoInstalledPackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No installed package with the given name found.")
    }
}

pub struct NoPackageError;
impl fmt::Debug for NoPackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No package with the given name found.")
    }
}

pub struct NoValidDistError;
impl fmt::Debug for NoValidDistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No valid distributable found for the package.")
    }
}
pub struct PackageAlreadyInstalledError;
impl fmt::Debug for PackageAlreadyInstalledError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The package is already installed.")
    }
}

pub struct FailedToUninstallError;
impl fmt::Debug for FailedToUninstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Auto-uninstallation failed. Manually uninstall the package and use --force flag to delete it from the database.")
    }
}

pub struct AlreadyUptoDateError;
impl fmt::Debug for AlreadyUptoDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The package is already up to date.")
    }
}
pub struct VersionAlreadyInstalledError;
impl fmt::Debug for VersionAlreadyInstalledError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The version of the package is already installed.")
    }
}

pub struct NoExecutableError;
impl fmt::Debug for NoExecutableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No executable found for the package.")
    }
}

pub enum KnownErrors {
    RequestError(reqwest::Error),
    IoError(io::Error),
    DatabaseError(tinydb::error::DatabaseError),
    PrivilegeError(PrivilegeError),
    RequestIoError(RequestIoError),
    RequestIoContentLengthError(RequestIoContentLengthError),
    NoExecutableError(NoExecutableError),
    VersionAlreadyInstalledError(VersionAlreadyInstalledError),
    AlreadyUptoDateError(AlreadyUptoDateError),
    FailedToUninstallError(FailedToUninstallError),
    NoInstalledPackageError(NoInstalledPackageError),
    NoPackageError(NoPackageError),
    NoValidDistError(NoValidDistError),
    PackageAlreadyInstalledError(PackageAlreadyInstalledError),
    ContentLengthError(ContentLengthError),
    NetworkError(NetworkError),
    ZipIoExeError(ZipIoExeError),
    SerdeError(serde_json::error::Error),
    ExportFileNotFoundError(ExportFileNotFoundError),
}

impl fmt::Debug for KnownErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KnownErrors::NoExecutableError(err) => write!(f, "{:?}", err),
            KnownErrors::RequestError(err) => write!(f, "{:?}", err),
            KnownErrors::IoError(err) => write!(f, "{:?}", err),
            KnownErrors::DatabaseError(err) => write!(f, "{:?}", err),
            KnownErrors::PrivilegeError(err) => write!(f, "{:?}", err),
            KnownErrors::RequestIoError(err) => write!(f, "{:?}", err),
            KnownErrors::RequestIoContentLengthError(err) => write!(f, "{:?}", err),
            KnownErrors::VersionAlreadyInstalledError(err) => write!(f, "{:?}", err),
            KnownErrors::AlreadyUptoDateError(err) => write!(f, "{:?}", err),
            KnownErrors::FailedToUninstallError(err) => write!(f, "{:?}", err),
            KnownErrors::NoInstalledPackageError(err) => write!(f, "{:?}", err),
            KnownErrors::NoPackageError(err) => write!(f, "{:?}", err),
            KnownErrors::NoValidDistError(err) => write!(f, "{:?}", err),
            KnownErrors::PackageAlreadyInstalledError(err) => write!(f, "{:?}", err),
            KnownErrors::ContentLengthError(err) => write!(f, "{:?}", err),
            KnownErrors::NetworkError(err) => write!(f, "{:?}", err),
            KnownErrors::ZipIoExeError(err) => write!(f, "{:?}", err),
            KnownErrors::SerdeError(err) => write!(f, "{:?}", err),
            KnownErrors::ExportFileNotFoundError(err) => write!(f, "{:?}", err),
        }
    }
}

pub enum ZipIoExeError {
    IoError(io::Error),
    ZipError(zip::result::ZipError),
    NoExeFouundError(NoExeFoundError),
}

impl fmt::Debug for ZipIoExeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ZipIoExeError::IoError(err) => write!(f, "{:?}", err),
            ZipIoExeError::ZipError(err) => write!(f, "{:?}", err),
            ZipIoExeError::NoExeFouundError(err) => write!(f, "{:?}", err),
        }
    }
}

impl From<io::Error> for ZipIoExeError {
    fn from(error: io::Error) -> Self {
        ZipIoExeError::IoError(error)
    }
}

impl From<zip::result::ZipError> for ZipIoExeError {
    fn from(error: zip::result::ZipError) -> Self {
        ZipIoExeError::ZipError(error)
    }
}

pub enum RequestIoContentLengthError {
    RequestIoError(RequestIoError),
    ContentLengthError(ContentLengthError),
}

impl fmt::Debug for RequestIoContentLengthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RequestIoContentLengthError::RequestIoError(err) => write!(f, "{:?}", err),
            RequestIoContentLengthError::ContentLengthError(err) => write!(f, "{:?}", err),
        }
    }
}

pub enum RequestIoError {
    IoError(io::Error),
    RequestError(reqwest::Error),
}

impl fmt::Debug for RequestIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestIoError::IoError(err) => write!(f, "{:?}", err),
            RequestIoError::RequestError(err) => write!(f, "{:?}", err),
        }
    }
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

impl From<ZipIoExeError> for KnownErrors {
    fn from(error: ZipIoExeError) -> Self {
        KnownErrors::ZipIoExeError(error)
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
impl From<ExportFileNotFoundError> for KnownErrors {
    fn from(err: ExportFileNotFoundError) -> Self {
        KnownErrors::ExportFileNotFoundError(err)
    }
}

impl From<RequestIoError> for KnownErrors {
    fn from(error: RequestIoError) -> Self {
        KnownErrors::RequestIoError(error)
    }
}

impl From<serde_json::error::Error> for KnownErrors {
    fn from(error: serde_json::error::Error) -> Self {
        KnownErrors::SerdeError(error)
    }
}

impl From<RequestIoContentLengthError> for KnownErrors {
    fn from(error: RequestIoContentLengthError) -> Self {
        KnownErrors::RequestIoContentLengthError(error)
    }
}

impl From<NoExecutableError> for KnownErrors {
    fn from(error: NoExecutableError) -> Self {
        KnownErrors::NoExecutableError(error)
    }
}

impl From<NoInstalledPackageError> for KnownErrors {
    fn from(error: NoInstalledPackageError) -> Self {
        KnownErrors::NoInstalledPackageError(error)
    }
}
impl From<VersionAlreadyInstalledError> for KnownErrors {
    fn from(error: VersionAlreadyInstalledError) -> Self {
        KnownErrors::VersionAlreadyInstalledError(error)
    }
}

impl From<AlreadyUptoDateError> for KnownErrors {
    fn from(error: AlreadyUptoDateError) -> Self {
        KnownErrors::AlreadyUptoDateError(error)
    }
}
impl From<FailedToUninstallError> for KnownErrors {
    fn from(error: FailedToUninstallError) -> Self {
        KnownErrors::FailedToUninstallError(error)
    }
}

impl From<NoPackageError> for KnownErrors {
    fn from(error: NoPackageError) -> Self {
        KnownErrors::NoPackageError(error)
    }
}

impl From<NoValidDistError> for KnownErrors {
    fn from(error: NoValidDistError) -> Self {
        KnownErrors::NoValidDistError(error)
    }
}

impl From<PackageAlreadyInstalledError> for KnownErrors {
    fn from(error: PackageAlreadyInstalledError) -> Self {
        KnownErrors::PackageAlreadyInstalledError(error)
    }
}

impl From<ContentLengthError> for KnownErrors {
    fn from(error: ContentLengthError) -> Self {
        KnownErrors::ContentLengthError(error)
    }
}

impl From<NetworkError> for KnownErrors {
    fn from(error: NetworkError) -> Self {
        KnownErrors::NetworkError(error)
    }
}

pub fn check_for_other_errors(err: KnownErrors) -> KnownErrors {
    let str_error = format!("{:?}", err);
    if str_error.contains("The requested operation requires elevation.") {
        return PrivilegeError.into(); // Happens when they disconnect for a decent while during an ongoing download
    } else if str_error.contains("No such host is known.") || str_error.contains("IncompleteBody") {
        return NetworkError.into();
    }
    err
}

pub fn print_error(err: KnownErrors) {
    let err = check_for_other_errors(err);
    eprintln!("\n{:?}", err);
}

