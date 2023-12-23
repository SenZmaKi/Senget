//!Manages senget package

use std::{io, path::PathBuf};

use regex::Regex;
use reqwest::Client;
use tinydb::error::DatabaseError;

use super::{
    database::PackageDBManager,
    error::KnownErrors,
    github::api::Repo,
    install::{InstallInfo, Installer},
    package::Package,
};

pub fn generate_senget_package(
    root_dir: PathBuf,
    version: String,
    description: String,
) -> Result<Package, io::Error> {
    let repo = Repo::new(
        "Senget".to_owned(),
        "SenZmaKi/Senpwai".to_owned(),
        "https://github.com/SenZmaKi/Senget".to_owned(),
        Some(description),
        Some("Rust".to_owned()),
        Some("GNU General Public License v3.0".to_owned()),
    );
    let executable_path = Some(root_dir.join("senget.exe"));
    let uninstall_command = Installer::fetch_uninstall_command_from_executable(&root_dir)?;
    let installation_folder = Some(root_dir);
    let install_info = InstallInfo {
        executable_path,
        installation_folder,
        uninstall_command,
    };
    Ok(Package::new(version, repo, install_info))
}

pub fn setup_senget_package(
    db: &mut PackageDBManager,
    senget_package: &Package, /*
                                The chance of the package being outdated or for the current execution to be the first run are way
                                lower than for this to be a normal run so instead of needlessly copying senget_package every time
                              this function is called we use reference such that we'll only copy it internally incase the aforementioned conditions are met
                              */
    version: &str,
) -> Result<(), DatabaseError> {
    match db.find_package("Senget")? {
        Some(old_senget_package) => {
            if old_senget_package.version != version {
                let old_senget_package = &old_senget_package.to_owned();
                db.update_package(old_senget_package, senget_package.to_owned())?;
            };
            Ok(())
        }
        None => {
            db.add_package(senget_package.to_owned())?;
            Ok(())
        }
    }
}

pub async fn check_if_senget_update_available(
    senget_package: &Package,
    client: &Client,
    version_regex: &Regex,
) -> Result<bool, reqwest::Error> {
    let latest_installer = senget_package
        .repo
        .get_latest_installer(client, version_regex)
        .await?;
    Ok(latest_installer.is_some())
}

