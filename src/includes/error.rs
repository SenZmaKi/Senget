//! Contains error handling utility

// I still don't understand the proper way to handle errors this language

use core::panic;
use mslnk::MSLinkError;
use reqwest;
use std::fmt;
use std::io;
use zip::result::ZipError;

use crate::eprintln_pretty;


pub struct ExportFileNotFoundError;

impl fmt::Debug for ExportFileNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Export file not found")
    }
}

pub struct NoExeFoundInZipError;
impl fmt::Debug for NoExeFoundInZipError {
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

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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
        write!(f, "Auto-uninstallation failed. Manually uninstall the package and use --force flag to delete it from the package database.")
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
    SerdeError(serde_json::error::Error),
    MSLinkError(MSLinkError),
    ZipError(ZipError),

    NetworkError(NetworkError),
    PrivilegeError(PrivilegeError),
    NoExecutableError(NoExecutableError),
    NoInstalledPackageError(NoInstalledPackageError),
    FailedToUninstallError(FailedToUninstallError),
    AlreadyUptoDateError(AlreadyUptoDateError),
    VersionAlreadyInstalledError(VersionAlreadyInstalledError),
    NoPackageError(NoPackageError),
    NoValidDistError(NoValidDistError),
    PackageAlreadyInstalledError(PackageAlreadyInstalledError),
    NoExeFound(NoExeFoundInZipError),
    ExportFileNotFoundError(ExportFileNotFoundError),
}

impl fmt::Debug for SengetErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SengetErrors::NoExecutableError(err) => write!(f, "{:?}", err),
            SengetErrors::RequestError(err) => write!(f, "{:?}", err),
            SengetErrors::IoError(err) => write!(f, "{:?}", err),
            SengetErrors::PrivilegeError(err) => write!(f, "{:?}", err),
            SengetErrors::VersionAlreadyInstalledError(err) => write!(f, "{:?}", err),
            SengetErrors::AlreadyUptoDateError(err) => write!(f, "{:?}", err),
            SengetErrors::FailedToUninstallError(err) => write!(f, "{:?}", err),
            SengetErrors::NoInstalledPackageError(err) => write!(f, "{:?}", err),
            SengetErrors::NoPackageError(err) => write!(f, "{:?}", err),
            SengetErrors::NoValidDistError(err) => write!(f, "{:?}", err),
            SengetErrors::PackageAlreadyInstalledError(err) => write!(f, "{:?}", err),
            SengetErrors::NetworkError(err) => write!(f, "{:?}", err),
            SengetErrors::NoExeFound(err) => write!(f, "{:?}", err),
            SengetErrors::SerdeError(err) => write!(f, "{:?}", err),
            SengetErrors::ExportFileNotFoundError(err) => write!(f, "{:?}", err),
            SengetErrors::MSLinkError(err) => write!(f, "{:?}", err),
            SengetErrors::ZipError(err) => write!(f, "{:?}", err),
        }
    }
}
impl fmt::Display for SengetErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for SengetErrors {}
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
impl From<serde_json::Error> for SengetErrors {
    fn from(err: serde_json::Error) -> Self {
        SengetErrors::SerdeError(err)
    }
}
impl From<NoExecutableError> for SengetErrors {
    fn from(err: NoExecutableError) -> Self {
        SengetErrors::NoExecutableError(err)
    }
}
impl From<FailedToUninstallError> for SengetErrors {
    fn from(err: FailedToUninstallError) -> Self {
        SengetErrors::FailedToUninstallError(err)
    }
}

impl From<VersionAlreadyInstalledError> for SengetErrors {
    fn from(err: VersionAlreadyInstalledError) -> Self {
        SengetErrors::VersionAlreadyInstalledError(err)
    }
}

impl From<AlreadyUptoDateError> for SengetErrors {
    fn from(err: AlreadyUptoDateError) -> Self {
        SengetErrors::AlreadyUptoDateError(err)
    }
}

impl From<NoInstalledPackageError> for SengetErrors {
    fn from(err: NoInstalledPackageError) -> Self {
        SengetErrors::NoInstalledPackageError(err)
    }
}

impl From<NoPackageError> for SengetErrors {
    fn from(err: NoPackageError) -> Self {
        SengetErrors::NoPackageError(err)
    }
}

impl From<NoValidDistError> for SengetErrors {
    fn from(err: NoValidDistError) -> Self {
        SengetErrors::NoValidDistError(err)
    }
}

impl From<PackageAlreadyInstalledError> for SengetErrors {
    fn from(err: PackageAlreadyInstalledError) -> Self {
        SengetErrors::PackageAlreadyInstalledError(err)
    }
}

impl From<NetworkError> for SengetErrors {
    fn from(err: NetworkError) -> Self {
        SengetErrors::NetworkError(err)
    }
}

impl From<MSLinkError> for SengetErrors {
    fn from(err: MSLinkError) -> Self {
        SengetErrors::MSLinkError(err)
    }
}
impl From<ZipError> for SengetErrors {
    fn from(err: ZipError) -> Self {
        SengetErrors::ZipError(err)
    }
}
impl From<NoExeFoundInZipError> for SengetErrors {
    fn from(err: NoExeFoundInZipError) -> Self {
        SengetErrors::NoExeFound(err)
    }
}

pub fn check_for_other_errors(err: SengetErrors) -> SengetErrors {
    match err {
        SengetErrors::IoError(io_err) => {
            if let io::ErrorKind::PermissionDenied = io_err.kind() {
                return PrivilegeError.into();
            }
            io_err.into()
        }
        SengetErrors::RequestError(req_err) => {
            let str_error = req_err.to_string();
            if str_error.contains("No such host is known.") || str_error.contains("IncompleteBody")
            {
                return NetworkError.into();
            }
            req_err.into()
        }
        _ => err,
    }
}

pub fn print_error(err: SengetErrors) {
    let err = check_for_other_errors(err);
    match err {
        SengetErrors::RequestError(err) => panic!("{}", err),
        SengetErrors::IoError(err) => panic!("{}", err),
        SengetErrors::SerdeError(err) => panic!("{}", err),
        SengetErrors::MSLinkError(err) => panic!("{}", err),
        SengetErrors::ZipError(err) => panic!("{}", err),
        _ => eprintln_pretty!("{}", err),
    }
}
