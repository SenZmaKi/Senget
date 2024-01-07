//!Manages installed package uninstallation and update

use crate::{dist::InstallInfo, github::api::Repo};
use core::fmt;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::{io, process::Command};
use winreg::RegKey;

use crate::includes::{
    dist::Dist,
    utils::{PathStr, MSI_EXEC},
};

use super::dist::{DistType, StartmenuFolders};
use super::error::KnownErrors;
use super::senget_manager::env::remove_package_folder_from_senget_env_var;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedPackage {
    pub lowercase_fullname: String,
    pub version: String,
    pub preferred_dist_type: DistType,
    pub create_shorcut_file: bool,
}

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
    pub fn export(&self) -> ExportedPackage {
        ExportedPackage {
            lowercase_fullname: self.lowercase_fullname.clone(),
            version: self.version.clone(),
            preferred_dist_type: self.install_info.dist_type.clone(),
            create_shorcut_file: self.install_info.create_shortcut_file
        }
    }
    pub fn installation_folder_str(&self) -> String {
        self.install_info
            .installation_folder
            .as_ref()
            .map(|f| f.path_str().unwrap_or_default())
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
                let program = split.next().unwrap_or_default().replace('"', "");
                // "/currentuser /S"
                let args_string = split.next().unwrap_or_default();
                // ["/currentuser", "/S"]
                let args = args_string.split(" - ").collect::<Vec<&str>>();
                (program, args)
            }
        }
    }
    pub fn uninstall(&self, startmenu_appdata_folder: &Path) -> Result<bool, io::Error> {
        if let Some(installation_folder) = self.install_info.installation_folder.as_ref() {
            remove_package_folder_from_senget_env_var(
                &installation_folder.path_str().unwrap_or_default(),
            )?
        };
        if self.install_info.dist_type == DistType::Installer {
            return self.uninstall_installer_distributable();
        };
        let installation_folder = self.install_info.installation_folder.as_ref().unwrap();
        if installation_folder.is_dir() {
            fs::remove_dir_all(installation_folder)?;
        }
        let shortcut_file_path = startmenu_appdata_folder.join(format!("{}.lnk", self.repo.name));
        if shortcut_file_path.is_file() {
            fs::remove_file(shortcut_file_path)?;
        }
        Ok(true)
    }
    fn uninstall_installer_distributable(&self) -> Result<bool, io::Error> {
        match &self.install_info.uninstall_command {
            Some(us) => {
                let (program, args) = Package::extract_program_and_args(us);
                if let Err(err) = Command::new(program).args(args).output() {
                    // TODO: Change this to err.kind() == io::Error::ErrorKind::InvalidFileName when it becomes stable
                    if err.to_string().contains(
                        "The filename, directory name, or volume label syntax is incorrect.",
                    ) {
                        // Assume that if the command didn't work then the user previously uninstalled it themselves
                        return Ok(false);
                    }
                }
                if let Some(executable_path) = self.install_info.executable_path.as_ref() {
                    if executable_path.is_file() {
                        return Ok(false)
                    }
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }
    pub async fn get_dist(
        &self,
        version: &str,
        client: &Client,
        version_regex: &Regex,
    ) -> Result<Option<Dist>, reqwest::Error> {
        match version {
            "latest" => {
                self.repo
                    .get_latest_dist(
                        client,
                        version_regex,
                        &Some(self.install_info.dist_type.clone()),
                    )
                    .await
            }
            version => {
                self.repo
                    .get_dist(
                        client,
                        version,
                        version_regex,
                        &Some(self.install_info.dist_type.clone()),
                    )
                    .await
            }
        }
    }

    pub fn install_updated_version(
        &self,
        dist: Dist,
        downloaded_dist_path: &Path,
        packages_folder_path: &Path,
        startmenu_folders: &StartmenuFolders,
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<Package, KnownErrors> {
        /* Generation of InstallInfo is pretty wonky, for the execuable_path it checks for
        new shortcut files after installation and for uninstall_command it checks for new registry entries.
        For these reasons there won't probably be any new shortcut files/registry entries if it's an update cause
        the update will just overwride the previously existing shortcut file/registry entry*/
        let (install_info, version) = match dist {
            Dist::Installer(dist) => (
                dist.install(
                    downloaded_dist_path,
                    self.install_info.create_shortcut_file,
                    startmenu_folders,
                    user_uninstall_reg_key,
                    machine_uninstall_reg_key,
                )?,
                dist.package_info.version,
            ),
            Dist::Exe(dist) => (
                dist.install(downloaded_dist_path, packages_folder_path, self.install_info.create_shortcut_file)?,
                dist.package_info.version,
            ),
            Dist::Zip(dist) => (
                dist.install(downloaded_dist_path,  packages_folder_path, self.install_info.create_shortcut_file)?,
                dist.package_info.version,
            ),
        };
        let executable_path = install_info
            .executable_path
            .or(self.install_info.executable_path.clone());
        let installation_folder = install_info
            .installation_folder
            .or(self.install_info.installation_folder.clone());
        let uninstall_command = install_info
            .uninstall_command
            .or(self.install_info.uninstall_command.clone());
        let preferred_dist_type = install_info.dist_type;
        Ok(Package::new(
            version,
            self.repo.clone(),
            InstallInfo {
                executable_path,
                installation_folder,
                uninstall_command,
                dist_type: preferred_dist_type,
                create_shortcut_file: self.install_info.create_shortcut_file,
            },
        ))
    }
}

