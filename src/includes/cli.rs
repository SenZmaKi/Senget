use std::path::PathBuf;

use regex::Regex;
use reqwest::Client;
use winreg::RegKey;

use crate::includes::{
    database::PackageDBManager,
    error::print_error_and_exit,
    github::{self, api::Repo},
};

use super::{error::KnownErrors, package::Package, utils::LoadingAnimation};

pub async fn find_repo(name: &str, client: &Client) -> Result<Option<Repo>, reqwest::Error> {
    let name_lower = name.to_lowercase();
    let mut found_repo: Option<Repo> = None;
    for r in github::api::search(name, client).await? {
        if r.full_name.to_lowercase() == name_lower {
            found_repo = Some(r)
        } else if found_repo.is_none() && r.name.to_lowercase() == name_lower {
            found_repo = Some(r)
        }
    }
    Ok(found_repo)
}

pub async fn show_package(
    db: &PackageDBManager,
    name: &str,
    client: &Client,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(package) => Ok(println!("{}", package)),
        None => match find_repo(name, client).await? {
            Some(repo) => Ok(println!("{}", repo)),
            None => Ok(println!("Couldn't find any package named \"{}\"", name)),
        },
    }
}

pub async fn install_package(
    db: &mut PackageDBManager,
    name: &str,
    client: &Client,
    version: &str,
    version_regex: &Regex,
    download_path: &PathBuf,
    loading_animation: &LoadingAnimation,
    startmenu_folder: &PathBuf,
    user_uninstall_reg_key: &RegKey,
    machine_uninstall_reg_key: &RegKey,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(package) => Ok(println!("{}\nPackage is already installed", package)),
        None => match find_repo(name, client).await? {
            Some(repo) => {
                let installer = match version {
                    "latest" => repo.get_latest_installer(client, version_regex).await?,
                    version => repo.get_installer(client, version, version_regex).await?,
                };
                match installer {
                    Some(installer) => {
                        let installer_path = installer.download(download_path, client).await?;
                        let install_info = installer.install(
                            &installer_path,
                            loading_animation,
                            startmenu_folder,
                            user_uninstall_reg_key,
                            machine_uninstall_reg_key,
                        )?;
                        let package_name = repo.name.to_owned();
                        let package = Package::new(installer.version, repo, install_info);
                        db.add_package(package)?;
                        Ok(println!("Succesfully installed {}", package_name))
                    }
                    None => Ok(println!("Couldn't find a valid installer for the package")),
                }
            }
            None => Ok(println!("Couldn't find a package named \"{}\"", name)),
        },
    }
}

mod tests {
    use crate::includes::{
        cli::show_package,
        test_utils::{client, db_manager, senpwai_latest_package},
    };

    #[tokio::test]
    async fn test_show_package() {
        let mut dbm = db_manager();
        let client = &client();
        show_package(&dbm, "Senpwai", client).await.unwrap();
        dbm.add_package(senpwai_latest_package()).unwrap();
        println!();
        show_package(&dbm, "SenZmaKi/Senpwai", client).await.unwrap();
        println!();
        show_package(&dbm, "99419gb0", client).await.unwrap();
    }
}
