//! Manages installed package uninstallation and update

use crate::{
    github::{self, api::Repo, serde_json_types::RepoResponseJson},
    install::InstallInfo,
    utils::{LoadingAnimation, RequestOrIOError},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf, process::Command};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub version: String,
    pub repo: Repo,
    install_info: InstallInfo,
}

impl Package {
    pub fn new(version: String, repo: Repo, install_info: InstallInfo) -> Package {
        Package {
            version,
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
    ) -> Result<Option<Package>, RequestOrIOError> {
        let installer = self
            .repo
            .get_latest_installer(client)
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
                let install_info = i.install(&p, loading_animation)?;
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
    use crate::includes::utils::{
        setup_client, LOADING_ANIMATION, PACKAGE_INSTALLER_DIR, SENPWAI_PACKAGE,
    };
    use tokio;

    #[tokio::test]
    async fn test_updating() {
        let package = SENPWAI_PACKAGE
            .update(&setup_client(), &PACKAGE_INSTALLER_DIR, &LOADING_ANIMATION)
            .await
            .expect("Successful Update")
            .expect("Updated Package");
        println!("Results for update {:?}", package);
        assert!(package.version != "2.0.6");
        assert!(package
            .install_info
            .executable_path
            .expect("Valid executable path")
            .is_file());
    }
    #[test]
    fn test_uninstalling() {
        assert!(SENPWAI_PACKAGE.uninstall(&LOADING_ANIMATION).unwrap())
    }
}
