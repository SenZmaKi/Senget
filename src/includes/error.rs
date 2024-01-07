//! Contains error handling utility

use mslnk::MSLinkError;
use reqwest;
use std::fmt;
use std::io;

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
        write!(f, "No executable found in the unpacked zip file")
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

pub enum SengetErrors {
    RequestError(reqwest::Error),
    IoError(io::Error),
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
    MSLinkError(MSLinkError)
}

impl fmt::Debug for SengetErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SengetErrors::NoExecutableError(err) => write!(f, "{:?}", err),
            SengetErrors::RequestError(err) => write!(f, "{:?}", err),
            SengetErrors::IoError(err) => write!(f, "{:?}", err),
            SengetErrors::PrivilegeError(err) => write!(f, "{:?}", err),
            SengetErrors::RequestIoError(err) => write!(f, "{:?}", err),
            SengetErrors::RequestIoContentLengthError(err) => write!(f, "{:?}", err),
            SengetErrors::VersionAlreadyInstalledError(err) => write!(f, "{:?}", err),
            SengetErrors::AlreadyUptoDateError(err) => write!(f, "{:?}", err),
            SengetErrors::FailedToUninstallError(err) => write!(f, "{:?}", err),
            SengetErrors::NoInstalledPackageError(err) => write!(f, "{:?}", err),
            SengetErrors::NoPackageError(err) => write!(f, "{:?}", err),
            SengetErrors::NoValidDistError(err) => write!(f, "{:?}", err),
            SengetErrors::PackageAlreadyInstalledError(err) => write!(f, "{:?}", err),
            SengetErrors::ContentLengthError(err) => write!(f, "{:?}", err),
            SengetErrors::NetworkError(err) => write!(f, "{:?}", err),
            SengetErrors::ZipIoExeError(err) => write!(f, "{:?}", err),
            SengetErrors::SerdeError(err) => write!(f, "{:?}", err),
            SengetErrors::ExportFileNotFoundError(err) => write!(f, "{:?}", err),
            SengetErrors::MSLinkError(err) => write!(f, "{:?}", err),
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

impl From<ZipIoExeError> for SengetErrors {
    fn from(error: ZipIoExeError) -> Self {
        SengetErrors::ZipIoExeError(error)
    }
}

impl From<reqwest::Error> for SengetErrors {
    fn from(error: reqwest::Error) -> Self {
        SengetErrors::RequestError(error)
    }
}

impl From<PrivilegeError> for SengetErrors {
    fn from(error: PrivilegeError) -> Self {
        SengetErrors::PrivilegeError(error)
    }
}
impl From<io::Error> for SengetErrors {
    fn from(error: io::Error) -> Self {
        SengetErrors::IoError(error)
    }
}

impl From<ExportFileNotFoundError> for SengetErrors {
    fn from(err: ExportFileNotFoundError) -> Self {
        SengetErrors::ExportFileNotFoundError(err)
    }
}

impl From<RequestIoError> for SengetErrors {
    fn from(error: RequestIoError) -> Self {
        SengetErrors::RequestIoError(error)
    }
}

impl From<serde_json::error::Error> for SengetErrors {
    fn from(error: serde_json::error::Error) -> Self {
        SengetErrors::SerdeError(error)
    }
}

impl From<RequestIoContentLengthError> for SengetErrors {
    fn from(error: RequestIoContentLengthError) -> Self {
        SengetErrors::RequestIoContentLengthError(error)
    }
}

impl From<NoExecutableError> for SengetErrors {
    fn from(error: NoExecutableError) -> Self {
        SengetErrors::NoExecutableError(error)
    }
}

impl From<NoInstalledPackageError> for SengetErrors {
    fn from(error: NoInstalledPackageError) -> Self {
        SengetErrors::NoInstalledPackageError(error)
    }
}
impl From<VersionAlreadyInstalledError> for SengetErrors {
    fn from(error: VersionAlreadyInstalledError) -> Self {
        SengetErrors::VersionAlreadyInstalledError(error)
    }
}

impl From<AlreadyUptoDateError> for SengetErrors {
    fn from(error: AlreadyUptoDateError) -> Self {
        SengetErrors::AlreadyUptoDateError(error)
    }
}
impl From<FailedToUninstallError> for SengetErrors {
    fn from(error: FailedToUninstallError) -> Self {
        SengetErrors::FailedToUninstallError(error)
    }
}

impl From<NoPackageError> for SengetErrors {
    fn from(error: NoPackageError) -> Self {
        SengetErrors::NoPackageError(error)
    }
}

impl From<NoValidDistError> for SengetErrors {
    fn from(error: NoValidDistError) -> Self {
        SengetErrors::NoValidDistError(error)
    }
}

impl From<PackageAlreadyInstalledError> for SengetErrors {
    fn from(error: PackageAlreadyInstalledError) -> Self {
        SengetErrors::PackageAlreadyInstalledError(error)
    }
}

impl From<ContentLengthError> for SengetErrors {
    fn from(error: ContentLengthError) -> Self {
        SengetErrors::ContentLengthError(error)
    }
}

impl From<NetworkError> for SengetErrors {
    fn from(error: NetworkError) -> Self {
        SengetErrors::NetworkError(error)
    }
}
impl From<MSLinkError> for SengetErrors {
    fn from(error: MSLinkError) -> Self {
        SengetErrors::MSLinkError(error)
    }
}

pub fn check_for_other_errors(err: SengetErrors) -> SengetErrors {
    let str_error = format!("{:?}", err);
    if str_error.contains("The requested operation requires elevation.") {
        return PrivilegeError.into(); // Happens when they disconnect for a decent while during an ongoing download
    } else if str_error.contains("No such host is known.") || str_error.contains("IncompleteBody") {
        return NetworkError.into();
    }
    err
}

pub fn print_error(err: SengetErrors) {
    let err = check_for_other_errors(err);
    eprintln!("\n{:?}", err);
}

