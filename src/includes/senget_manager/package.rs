//!Manages senget package

use crate::includes::{
    database::PackageDatabase,
    dist::{DistType, InstallInfo, InstallerDist},
    error::SengetErrors,
    github::api::Repo,
    package::Package,
    utils::{DESCRIPTION, REPO_URL, VERSION},
};
use regex::Regex;
use reqwest::Client;
use std::{env, io};

pub fn generate_senget_package() -> Result<Package, io::Error> {
    let repo = Repo::new(
        "Senget".to_owned(),
        "SenZmaKi/Senget".to_owned(),
        REPO_URL.to_owned(),
        Some(DESCRIPTION.to_owned()),
        Some("Rust".to_owned()),
        Some("GNU General Public License v3.0".to_owned()),
    );
    let some_executable_path = env::current_exe().unwrap();
    let some_installation_folder = some_executable_path.parent().unwrap().to_path_buf();
    let uninstall_command =
        InstallerDist::fetch_uninstall_command_from_executable(&some_installation_folder)?;
    let executable_path = Some(some_executable_path);
    let installation_folder = Some(some_installation_folder);
    let install_info = InstallInfo {
        executable_path,
        installation_folder,
        uninstall_command,
        dist_type: DistType::Installer,
        create_shortcut_file: false,
    };
    Ok(Package::new(VERSION.to_owned(), repo, install_info))
}

pub fn setup_senget_package(
    db: &PackageDatabase,
    // The chance of the package being outdated or for the current execution to be the first run are way
    // lower than for this to be a normal run so instead of needlessly copying senget_package every time
    // this function is called we use a reference such that we'll only conpy it internally incase the
    // aforementioned conditions are met
    senget_package: &Package,
) -> Result<(), SengetErrors> {
    match db.find_package("Senget")? {
        Some(old_senget_package) => {
            if old_senget_package.version != VERSION {
                db.update_package(&old_senget_package, senget_package.clone())?;
            };
        }
        None => {
            db.add_package(senget_package.clone())?;
        }
    }
    Ok(())
}

pub async fn check_if_senget_update_available(
    senget_package: &Package,
    client: &Client,
    version_regex: &Regex,
) -> Result<bool, reqwest::Error> {
    let latest_dist = senget_package
        .repo
        .get_latest_dist(client, version_regex, &Some(DistType::Installer))
        .await?;
    if let Some(dist) = latest_dist {
        return Ok(dist.version() != senget_package.version);
    }
    Ok(false)
}
