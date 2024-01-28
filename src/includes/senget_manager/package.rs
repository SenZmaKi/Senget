//!Manages senget package

use crate::includes::{
    database::PackageDatabase,
    dist::{DistType, InstallInfo, InstallerDist},
    error::SengetErrors,
    github::api::Repo,
    package::Package,
};
use regex::Regex;
use reqwest::Client;
use std::{io, path::PathBuf};

pub fn generate_senget_package(
    root_dir: PathBuf,
    version: String,
    description: String,
) -> Result<Package, io::Error> {
    let repo = Repo::new(
        "Senget".to_owned(),
        "SenZmaKi/Senget".to_owned(),
        "https://github.com/SenZmaKi/Senget".to_owned(),
        Some(description),
        Some("Rust".to_owned()),
        Some("GNU General Public License v3.0".to_owned()),
    );
    let executable_path = Some(root_dir.join("senget.exe"));
    let uninstall_command = InstallerDist::fetch_uninstall_command_from_executable(&root_dir)?;
    let installation_folder = Some(root_dir);
    let install_info = InstallInfo {
        executable_path,
        installation_folder,
        uninstall_command,
        dist_type: DistType::Installer,
        create_shortcut_file: false,
    };
    Ok(Package::new(version, repo, install_info))
}

pub fn setup_senget_package(
    db: &PackageDatabase,
    // The chance of the package being outdated or for the current execution to be the first run are way
    // lower than for this to be a normal run so instead of needlessly copying senget_package every time
    // this function is called we use a reference such that we'll only copy it internally incase the
    // aforementioned conditions are met
    senget_package: &Package,
    version: &str,
) -> Result<(), SengetErrors> {
    match db.find_package("Senget")? {
        Some(old_senget_package) => {
            if old_senget_package.version != version {
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
