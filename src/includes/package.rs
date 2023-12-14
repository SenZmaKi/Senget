//! Manages installed package uninstallation and update

use crate::{github::api::Repo, install::InstallInfo};
use core::fmt;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf, process::Command};
use winreg::RegKey;

use crate::includes::error::RequestIoContentLengthError;

use super::{
    install::Installer,
    utils::{display_path, MSI_EXEC},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub version: String,
    pub lowercase_name: String, // Used when querying the database
    pub lowercase_fullname: String,
    pub repo: Repo,
    pub install_info: InstallInfo,
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\nVersion: {}\nInstallation Folder: {}",
            self.repo,
            &self.version,
            self.installation_folder_str()
        )
    }
}
impl Package {
    pub fn new(version: String, repo: Repo, install_info: InstallInfo) -> Package {
        Package {
            version,
            lowercase_name: repo.name.to_lowercase(),
            lowercase_fullname: repo.full_name.to_lowercase(),
            repo,
            install_info,
        }
    }
    pub fn installation_folder_str(&self) -> String {
        self.install_info
            .installation_folder
            .as_ref()
            .map(|f| display_path(f).unwrap_or_default())
            .unwrap_or_default()
    }

    // 🤓 "Umm actually if you use a regex it'll be faster and more readable", FUCK OFF!!!
    fn extract_program_and_args(uninstall_command: &str) -> (String, Vec<&str>) {
        match uninstall_command.contains(MSI_EXEC) {
            true => {
                let msi = &format!("{} ", MSI_EXEC);
                let mut split = uninstall_command.split(msi);
                let _ = split.next(); // Ignore the first value since it's just MSI_EXEC
                (MSI_EXEC.to_owned(), split.collect::<Vec<&str>>())
            }
            false => {
                // ""C:\Users\PC\AppData\Local\Programs\Miru\Uninstall Miru.exe" /currentuser /s"
                let mut split = uninstall_command.split("\" ");
                // "C:\Users\PC\AppData\Local\Programs\Miru\Uninstall Miru.exe"
                let program = split.next().unwrap_or_default().replace("\"", "");
                // "/currentuser /S"
                let args_string = split.next().unwrap_or_default();
                // ["/currentuser", "/S"]
                let args = args_string.split(" - ").collect::<Vec<&str>>();
                (program, args)
            }
        }
    }
    pub fn uninstall(&self) -> Result<bool, io::Error> {
        match &self.install_info.uninstall_command {
            Some(us) => {
                let (program, args) = Package::extract_program_and_args(us);
                if let Err(err) = Command::new(program).args(args).output() {
                    // TODO: Change this to err.kind() == io::Error::ErrorKind::InvalidFileName when it becomes stable
                    if err.to_string().contains(
                        "The filename, directory name, or volume label syntax is incorrect.",
                    ) {
                        // We assume that if the command didn't work then the user previously uninstalled it themselves
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }
    pub async fn get_installer(
        &self,
        version: &str,
        client: &Client,
        version_regex: &Regex,
    ) -> Result<Option<Installer>, reqwest::Error> {
        match version {
            "latest" => self.repo.get_latest_installer(client, version_regex).await,
            version => {
                self.repo
                    .get_installer(client, version, version_regex)
                    .await
            }
        }
    }

    pub fn install_updated_version(
        &self,
        installer: Installer,
        installer_path: &PathBuf,
        startmenu_folders: &(PathBuf, PathBuf),
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<Package, RequestIoContentLengthError> {
        /* Generation of InstallInfo is pretty wonky, for the execuable_path it checks for
        new shortcut files after installation and for uninstall_command it checks for new registry entries.
        For these reasons there won't probably be any new shortcut files/registry entries if it's an update cause
        the update will just overwride the previously existing shortcut file/registry entry*/
        let install_info = installer.install(
            &installer_path,
            startmenu_folders,
            user_uninstall_reg_key,
            machine_uninstall_reg_key,
        )?;
        let executable_path = install_info
            .executable_path
            .or(self.install_info.executable_path.to_owned());
        let installation_folder = install_info
            .installation_folder
            .or(self.install_info.installation_folder.to_owned());
        let uninstall_command = install_info
            .uninstall_command
            .or(self.install_info.uninstall_command.to_owned());
        Ok(Package::new(
            installer.version,
            self.repo.to_owned(),
            InstallInfo {
                executable_path,
                installation_folder,
                uninstall_command,
            },
        ))
    }
}
mod tests {
    use crate::includes::{
        github::api::Repo,
        install::Installer,
        test_utils::{client, package_installers_dir, senpwai_latest_package, senpwai_package},
    };
    #[test]
    fn test_uninstalling() {
        assert!(senpwai_latest_package().uninstall().expect("Ok(uninstall)"))
    }
}
