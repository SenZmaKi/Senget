mod includes;
use std::path::PathBuf;

use clap::Error;
use includes::{
    cli::{self, match_commands},
    database::PackageDBManager,
    error::{print_error, KnownErrors},
    github,
    install::{self, Installer},
    utils::{self, setup_client, LoadingAnimation},
};
use regex::Regex;
use tokio::runtime::Runtime;
use winreg::RegKey;

fn setup() -> Result<
    (
        PackageDBManager,
        Regex,
        reqwest::Client,
        PathBuf,
        LoadingAnimation,
        (PathBuf, PathBuf),
        RegKey,
        RegKey,
    ),
    KnownErrors,
> {
    let db_save_path = PackageDBManager::get_db_file_path()?;
    let db = PackageDBManager::new(&db_save_path)?;
    let version_regex = github::api::Repo::generate_version_regex();
    let client = setup_client()?;

    let installer_download_path = Installer::generate_installer_download_path()?;
    let loading_animation = LoadingAnimation::new();
    let startmenu_folders = Installer::generate_startmenu_paths();
    let user_uninstall_reg_key = Installer::generate_user_uninstall_reg_key()?;
    let machine_uninstall_reg_key = Installer::generate_machine_uninstall_reg_key()?;

    Ok((
        db,
        version_regex,
        client,
        installer_download_path,
        loading_animation,
        startmenu_folders,
        user_uninstall_reg_key,
        machine_uninstall_reg_key,
    ))
}

fn main() {
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            print_error(err.into());
            return;
        }
    };

    let commands = cli::parse_commands();

    rt.block_on(async {
        if let Err(err) = {
            let (
                mut db,
                version_regex,
                client,
                installer_download_path,
                loading_animation,
                startmenu_folder,
                user_uninstall_reg_key,
                machine_uninstall_reg_key,
            ) = match setup() {
                Ok(s) => s,
                Err(err) => {
                    print_error(err);
                    return;
                }
            };
            match_commands(
                commands,
                &mut db,
                &client,
                &installer_download_path,
                &version_regex,
                &loading_animation,
                &startmenu_folder,
                &user_uninstall_reg_key,
                &machine_uninstall_reg_key,
            )
            .await
        } {
            print_error(err);
        }
    });
}
