//! Manages installed package uninstallation and update

use crate::{
    github::{self, api::Repo, serde_json_types::RepoResponseJson},
    install::InstallInfo,
    utils::{GenericError, LoadingAnimation},
};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf, process::Command};
use winreg::RegKey;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub version: String,
    pub lowercase_name: String, // Used when querying the database
    pub repo: Repo,
    install_info: InstallInfo,
}

impl Package {
    pub fn new(version: String, repo: Repo, install_info: InstallInfo) -> Package {
        Package {
            version,
            lowercase_name: repo.name.to_lowercase(),
            repo,
            install_info,
        }
    }
    pub fn uninstall(&self, loading_animation: &LoadingAnimation) -> Result<bool, io::Error> {
        match &self.install_info.uninstall_command {
            Some(us) => {
                let join_handle = loading_animation.start(format!(
                    "Uninstalling {} v{}.. .",
                    self.repo.full_name, self.version
                ));
                Command::new(us).output()?;
                loading_animation.stop(join_handle);
                Ok(true)
            }
            None => Ok(false),
        }
    }
    pub async fn update(
        &self,
        client: &Client,
        path: &PathBuf,
        loading_animation: &LoadingAnimation,
        version_regex: &Regex,
        startmenu_folder: &PathBuf,
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<Option<Package>, GenericError> {
        let installer = self
            .repo
            .get_latest_installer(client, version_regex)
            .await?
            .filter(|i| i.version != self.version);
        match installer {
            Some(i) => {
                println!("Updating from {} --> {}", self.version, i.version);
                let p = i.download(path, client).await?;
                /* Generation of InstallInfo is pretty wonky, for the execuable_path it checks for
                new shorcut files after installation and for uninstall_command it checks for new registry entries.
                For these reasons there won't probably be any new shortcut files/registry entries if it's an update cause
                the update will just overwride the previously existing shortcut file/registry entry*/
                let install_info = i.install(
                    &p,
                    loading_animation,
                    startmenu_folder,
                    user_uninstall_reg_key,
                    machine_uninstall_reg_key,
                )?;
                let executable_path = install_info
                    .executable_path
                    .or(self.install_info.executable_path.clone());
                let uninstall_command = install_info
                    .uninstall_command
                    .or(self.install_info.uninstall_command.clone());
                let repo_response_json: RepoResponseJson =
                    client.get(&self.repo.url).send().await?.json().await?;
                Ok(Some(Package::new(
                    i.version,
                    github::api::extract_repo(repo_response_json),
                    InstallInfo {
                        executable_path,
                        uninstall_command,
                    },
                )))
            }
            None => Ok(None),
        }
    }
}

mod tests {
    use crate::includes::{
        github::api::Repo,
        install::Installer,
        utils::{setup_client, LOADING_ANIMATION, PACKAGE_INSTALLER_DIR, SENPWAI_PACKAGE},
    };
    use tokio;

    #[tokio::test]
    async fn test_updating() {
        let new_package = SENPWAI_PACKAGE
            .update(
                &setup_client().unwrap(),
                &PACKAGE_INSTALLER_DIR,
                &LOADING_ANIMATION,
                &Repo::generate_version_regex(),
                &Installer::generate_startmenu_path(),
                &Installer::generate_user_uninstall_reg_key().expect("Ok(user_uninstall_reg_key)"),
                &Installer::generate_machine_uninstall_reg_key()
                    .expect("Ok(machine_uninstall_reg_key)"),
            )
            .await
            .expect("Ok(Option<Package>)")
            .expect("Some(new_package)");
        println!("Results for update {:?}", new_package);
        assert!(new_package.version != "2.0.6");
        assert!(new_package
            .install_info
            .executable_path
            .expect("Some(executable_path)")
            .is_file());
        assert!(new_package.install_info.uninstall_command.is_some());
    }
    #[test]
    fn test_uninstalling() {
        assert!(SENPWAI_PACKAGE
            .uninstall(&LOADING_ANIMATION)
            .expect("Ok(uninstall)"))
    }
}
