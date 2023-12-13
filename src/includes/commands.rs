use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

use regex::Regex;
use reqwest::Client;
use winreg::RegKey;

use crate::includes::{
    database::PackageDBManager,
    error::{print_error, KnownErrors},
    github::{self, api::Repo},
    install::Installer,
    package::Package,
    utils::{LoadingAnimation, APP_NAME_LOWER, display_path},
};

fn eprint_no_installed_package_found(name: &str) {
    eprintln!("No installed package named \"{}\" found.", name);
}

fn eprint_no_package_found(name: &str) {
    eprintln!("No package named \"{}\" found.", name);
}

async fn find_repo(name: &str, client: &Client) -> Result<Option<Repo>, KnownErrors> {
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
            None => Ok(eprint_no_package_found(name)),
        },
    }
}

pub async fn download_installer(
    name: &str,
    client: &Client,
    version: &str,
    version_regex: &Regex,
    download_path: &PathBuf,
) -> Result<(), KnownErrors> {
    if let Some((_, _, installer_path)) =
        internal_download_installer(name, client, version, version_regex, download_path).await?
    {
        println!("Downloaded at {}", display_path(&installer_path)?);
    }
    Ok(())
}
async fn internal_download_installer(
    name: &str,
    client: &Client,
    version: &str,
    version_regex: &Regex,
    download_path: &PathBuf,
) -> Result<Option<(Repo, Installer, PathBuf)>, KnownErrors> {
    match find_repo(name, client).await? {
        Some(repo) => {
            let installer = match version {
                "latest" => repo.get_latest_installer(client, version_regex).await?,
                version => repo.get_installer(client, version, version_regex).await?,
            };
            match installer {
                Some(installer) => {
                    let installer_path = installer.download(download_path, client).await?;
                    Ok(Some((repo, installer, installer_path)))
                }
                None => {
                    eprint_no_installed_package_found(name);
                    Ok(None)
                }
            }
        }
        None => {
            eprint_no_package_found(name);
            Ok(None)
        }
    }
}
pub async fn install_package(
    db: &mut PackageDBManager,
    name: &str,
    client: &Client,
    version: &str,
    version_regex: &Regex,
    installer_download_path: &PathBuf,
    loading_animation: &LoadingAnimation,
    startmenu_folders: &(PathBuf, PathBuf),
    user_uninstall_reg_key: &RegKey,
    machine_uninstall_reg_key: &RegKey,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(package) => {
            println!("{}\n", package);
            Ok(eprintln!("Package is already installed."))
        }
        None => {
            match internal_download_installer(
                name,
                client,
                version,
                version_regex,
                installer_download_path,
            )
            .await?
            {
                Some((repo, installer, installer_path)) => {
                    let install_info = installer.install(
                        &installer_path,
                        loading_animation,
                        startmenu_folders,
                        user_uninstall_reg_key,
                        machine_uninstall_reg_key,
                    )?;
                    let package_name = repo.name.to_owned();
                    let package = Package::new(installer.version, repo, install_info);
                    db.add_package(package)?;
                    Ok(println!("Successfully installed {}.", package_name))
                }
                None => Ok(()),
            }
        }
    }
}

pub fn uninstall_package(
    db: &mut PackageDBManager,
    name: &str,
    loading_animation: &LoadingAnimation,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(package) => {
            let uninstalled = package.uninstall(loading_animation)?;
            let name = package.repo.name.to_owned();
            db.remove_package(&package.to_owned())?;
            match uninstalled {
                true => Ok(println!("Successfully uninstalled {}.", name)),
                false => Ok(eprintln!(
                    "Failed to automatically uninstall the package, but it was removed from the package database"
                )),
            }
        }
        None => Ok(eprint_no_installed_package_found(name)),
    }
}

pub async fn update_package(
    db: &mut PackageDBManager,
    name: &str,
    client: &Client,
    version: &str,
    version_regex: &Regex,
    installer_download_path: &PathBuf,
    loading_animation: &LoadingAnimation,
    startmenu_folders: &(PathBuf, PathBuf),
    user_uninstall_reg_key: &RegKey,
    machine_uninstall_reg_key: &RegKey,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(old_package) => {
            match old_package
                .update(
                    client,
                    installer_download_path,
                    loading_animation,
                    version,
                    version_regex,
                    startmenu_folders,
                    user_uninstall_reg_key,
                    machine_uninstall_reg_key,
                )
                .await?
            {
                Some(new_package) => {
                    db.update_package(&old_package.to_owned(), new_package)?;
                    Ok(())
                }
                None => Ok(eprintln!(
                    "No valid installer found for {}.",
                    old_package.repo.name
                )),
            }
        }
        None => Ok(eprint_no_installed_package_found(name)),
    }
}

pub fn list_packages(db: &PackageDBManager) -> () {
    let mut name_width = "Name".len();
    let mut version_width = "Version".len();
    let mut installation_folder_width = "Installation Folder".len();
    let compare_len = |prev_max_len: usize, curr_str: &str| curr_str.len().max(prev_max_len);
    let packages = db.fetch_all_packages();
    if packages.is_empty() {
        return println!("No packages installed");
    }
    for p in packages {
        name_width = compare_len(name_width, &p.repo.name);
        version_width = compare_len(version_width, &p.version);
        installation_folder_width =
            compare_len(installation_folder_width, &p.installation_folder_str());
    }
    let format_row = |name: &str, version: &str, installation_folder: &str| {
        format!(
            "{:<name_width$}    {:<version_width$}    {:<installation_folder_width$}\n",
            name, version, installation_folder
        )
    };
    let mut final_str = format_row("Name", "Version", "Installation Folder");
    let spaces_count = 4 + 4;
    final_str += &"-".repeat(name_width + version_width + installation_folder_width + spaces_count);
    final_str += "\n";
    for p in packages {
        final_str += &format_row(&p.repo.name, &p.version, &p.installation_folder_str());
    }
    print!("{}", final_str);
}

pub async fn search_repos(query: &str, client: &Client) -> Result<(), KnownErrors> {
    let results = github::api::search(query, client).await?;
    let mut final_str = "".to_owned();
    if results.is_empty() {
        return Ok(println!("No results found."));
    }
    for r in results {
        final_str += &format!(
            "Full Name: {}\nDescription: {}\n\n",
            r.full_name,
            r.description.unwrap_or_default()
        );
    }
    Ok(print!("{}", final_str))
}

pub fn exported_packages_file_name() -> String {
    format!("{}-packages.txt", APP_NAME_LOWER)
}
pub fn export_packages(
    db: &PackageDBManager,
    export_folder_path: &PathBuf,
) -> Result<(), KnownErrors> {
    let mut final_str = "".to_owned();
    for p in db.fetch_all_packages() {
        let p_entry = format!("{}=={}\n", &p.repo.full_name, &p.version);
        final_str.push_str(&p_entry);
    }
    let export_file_path = export_folder_path.join(exported_packages_file_name());
    let mut f = File::create(&export_file_path)?;
    f.write_all(final_str.as_bytes())?;
    Ok(println!(
        "Exported at {}",
        export_file_path.display().to_string()
    ))
}

pub async fn import_packages(
    export_file_path: &PathBuf,
    ignore_versions: bool,
    db: &mut PackageDBManager,
    client: &Client,
    version_regex: &Regex,
    installer_download_path: &PathBuf,
    loading_animation: &LoadingAnimation,
    startmenu_folders: &(PathBuf, PathBuf),
    user_uninstall_reg_key: &RegKey,
    machine_uninstall_reg_key: &RegKey,
) -> Result<(), KnownErrors> {
    for (name, version) in extract_package_name_and_version(export_file_path, ignore_versions)? {
        if let Err(err) = install_package(
            db,
            &name,
            client,
            &version,
            version_regex,
            installer_download_path,
            loading_animation,
            startmenu_folders,
            user_uninstall_reg_key,
            machine_uninstall_reg_key,
        )
        .await
        {
            print_error(err);
        }
    }
    Ok(())
}

fn extract_package_name_and_version(
    export_file_path: &PathBuf,
    ignore_versions: bool,
) -> Result<Vec<(String, String)>, KnownErrors> {
    let mut name_and_version: Vec<(String, String)> = Vec::new();
    for line in fs::read_to_string(export_file_path)?
        .lines()
        .into_iter()
        .filter(|l| !l.is_empty())
    {
        let mut package_name = line;
        let version = match ignore_versions {
            true => "latest",
            false => {
                let split = line.split("==").collect::<Vec<&str>>();
                match split.len() >= 2 {
                    true => {
                        package_name = split[0];
                        split[1]
                    }
                    false => "latest",
                }
            }
        };
        name_and_version.push((package_name.to_owned(), version.to_owned()));
    }
    Ok(name_and_version)
}

pub fn run_package(db: &PackageDBManager, name: &str) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(p) => match &p.install_info.executable_path {
            Some(ep) => {
                println!("Starting {}.. .", p.repo.name);
                Command::new(ep).spawn()?;
                Ok(())
            }
            None => Ok(eprintln!("No executable found for {}.", p.repo.name)),
        },
        None => Ok(eprint_no_installed_package_found(name)),
    }
}

mod tests {
    use crate::includes::{
        commands::{list_packages, search_repos, show_package},
        test_utils::{client, db_manager, hatt_package, senpwai_latest_package},
    };

    #[tokio::test]
    async fn test_show_package() {
        let mut db = db_manager();
        let client = &client();
        show_package(&db, "Senpwai", client).await.unwrap();
        println!();
        db.add_package(senpwai_latest_package()).unwrap();
        show_package(&db, "SenZmaKi/Senpwai", client).await.unwrap();
        println!();
        show_package(&db, "99419gb0", client).await.unwrap();
    }

    #[tokio::test]
    async fn test_search_repos() {
        search_repos("Python", &client()).await.unwrap();
    }

    #[test]
    fn test_list_packages() {
        let mut db = db_manager();
        db.add_package(senpwai_latest_package()).unwrap();
        db.add_package(hatt_package()).unwrap();
        list_packages(&db);
    }
}
