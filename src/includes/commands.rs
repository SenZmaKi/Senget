use std::{
    fs::{self, File},
    io::Write,
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
    utils::{display_path, loading_animation, setup_client, APP_NAME_LOWER},
};

pub struct Statics {
    pub db: PackageDBManager,
    pub client: Client,
    pub version_regex: Regex,
    pub installer_download_path: PathBuf,
    pub startmenu_folders: (PathBuf, PathBuf),
    pub user_uninstall_reg_key: RegKey,
    pub machine_uninstall_reg_key: RegKey,
}

impl Statics {
    pub fn new() -> Result<Statics, KnownErrors> {
        let db_save_path = PackageDBManager::get_db_file_path()?;
        let db = PackageDBManager::new(&db_save_path)?;
        let client = setup_client()?;
        let installer_download_path = Installer::generate_installer_download_path()?;
        let startmenu_folders = Installer::generate_startmenu_paths();
        let user_uninstall_reg_key = Installer::generate_user_uninstall_reg_key()?;
        let machine_uninstall_reg_key = Installer::generate_machine_uninstall_reg_key()?;
        let version_regex = github::api::Repo::generate_version_regex();
        Ok(Statics {
            db,
            client,
            installer_download_path,
            startmenu_folders,
            user_uninstall_reg_key,
            machine_uninstall_reg_key,
            version_regex,
        })
    }
}

fn eprintln_no_installed_package_found(name: &str) {
    eprintln!("No installed package named \"{}\" found.", name);
}

fn eprintln_no_package_found(name: &str) {
    eprintln!("No package named \"{}\" found.", name);
}

fn eprintln_no_valid_installer(package_name: &str) {
    eprintln!("No valid installer for {} was found", package_name);
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
    name: &str,
    db: &PackageDBManager,
    client: &Client,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(package) => Ok(println!("{}", package)),
        None => match find_repo(name, client).await? {
            Some(repo) => Ok(println!("{}", repo)),
            None => Ok(eprintln_no_package_found(name)),
        },
    }
}

pub fn clear_cached_installers(installer_folder_path: &PathBuf) -> Result<(), KnownErrors> {
    for f in installer_folder_path.read_dir()? {
        let f = f?.path();
        if f.is_file() {
            fs::remove_file(f)?;
        }
    }
    Ok(())
}

pub fn purge_packages(db: &mut PackageDBManager) -> Result<(), KnownErrors> {
    let mut to_remove: Vec<Package> = Vec::new();
    for p in db.fetch_all_packages() {
        if let Some(exe) = p.install_info.executable_path.as_ref() {
            if !exe.is_file() {
                to_remove.push(p.to_owned());
            }
        }
    }
    for p in to_remove {
        db.remove_package(&p)?;
        println!("Purged {}", p.repo.name);
    }
    Ok(())
}

async fn update_all_packages(version: &str, statics: &mut Statics) -> Result<(), KnownErrors> {
    let mut errored_packages: Vec<String> = Vec::new();
    for p in statics.db.fetch_all_packages().to_owned() {
        if let Err(err) = update_package(&p.repo.name, version, statics).await {
            errored_packages.push(p.repo.name);
        }
    }
    match errored_packages.is_empty() {
        true => println!("Successfully updated all the necessary packages"),
        false => println!(
            "Errors encountered updating the following packages:\n{}",
            errored_packages.join(", ")
        ),
    }
    Ok(())
}

pub async fn download_installer(
    name: &str,
    version: &str,
    download_path: &PathBuf,
    client: &Client,
    version_regex: &Regex,
) -> Result<(), KnownErrors> {
    if let Some((_, _, installer_path)) =
        internal_download_installer(name, version, client, version_regex, download_path).await?
    {
        println!("Downloaded at {}", display_path(&installer_path)?);
    }
    Ok(())
}
async fn internal_download_installer(
    name: &str,
    version: &str,
    client: &Client,
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
                    eprintln_no_valid_installer(&repo.name);
                    Ok(None)
                }
            }
        }
        None => {
            eprintln_no_package_found(name);
            Ok(None)
        }
    }
}
pub async fn install_package(
    name: &str,
    version: &str,
    statics: &mut Statics,
) -> Result<(), KnownErrors> {
    match statics.db.find_package(name)? {
        Some(package) => {
            println!("{}\n", package);
            Ok(eprintln!("Package is already installed."))
        }
        None => {
            match internal_download_installer(
                name,
                version,
                &statics.client,
                &statics.version_regex,
                &statics.installer_download_path,
            )
            .await?
            {
                Some((repo, installer, installer_path)) => {
                    let task = || {
                        installer.install(
                            &installer_path,
                            &statics.startmenu_folders,
                            &statics.user_uninstall_reg_key,
                            &statics.machine_uninstall_reg_key,
                        )
                    };
                    let install_info =
                        loading_animation(format!("Installing {}.. .", repo.name), task)?;
                    let package_name = repo.name.to_owned();
                    let package = Package::new(installer.version, repo, install_info);
                    statics.db.add_package(package)?;
                    Ok(println!("Successfully installed {}.", package_name))
                }
                None => Ok(()),
            }
        }
    }
}

pub fn uninstall_package(
    name: &str,
    force: bool,
    db: &mut PackageDBManager,
) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(package) => {
            let task = || package.uninstall();
            let uninstalled =
                loading_animation(format!("Uninstalling {}", package.repo.name), task)?;
            let name = package.repo.name.to_owned();
            db.remove_package(&package.to_owned())?;
            match uninstalled {
                true => Ok(println!("Successfully uninstalled {}.", name)),
                false => match force {
                    true => Ok(eprintln!(
                    "Failed to automatically uninstall the package, but it was removed from the package database"
                )),
                    false => Ok(eprintln!("Failed to automatically uninstall the package, manually uninstall it then run the uninstall command with --force flag to remove it from the package database"))},
            }
        }
        None => Ok(eprintln_no_installed_package_found(name)),
    }
}

pub async fn update_handler(
    name: &str,
    version: &str,
    statics: &mut Statics,
) -> Result<(), KnownErrors> {
    match name == "all" {
        true => update_all_packages("latest", statics).await,
        false => update_package(name, version, statics).await,
    }
}

async fn update_package(
    name: &str,
    version: &str,
    statics: &mut Statics,
) -> Result<(), KnownErrors> {
    match statics.db.find_package(name)? {
        Some(old_package) => {
            match old_package
                .get_installer(version, &statics.client, &statics.version_regex)
                .await?
            {
                Some(installer) => match old_package.version == installer.version {
                    true => {
                        match version == "latest" {
                            true => {
                                eprintln!("{} is already up to date", old_package.repo.name)
                            }
                            false => eprintln!(
                                "{} v{} is already installed",
                                old_package.repo.name, old_package.version
                            ),
                        };
                        Ok(())
                    }
                    false => {
                        println!(
                            "Updating {} from {} --> {}",
                            old_package.repo.name, old_package.version, installer.version
                        );
                        let installer_path = installer
                            .download(&statics.installer_download_path, &statics.client)
                            .await?;
                        let task = || {
                            old_package.install_updated_version(
                                installer,
                                &installer_path,
                                &statics.startmenu_folders,
                                &statics.user_uninstall_reg_key,
                                &statics.machine_uninstall_reg_key,
                            )
                        };
                        let new_package = loading_animation(
                            format!("Updating {}.. .", old_package.repo.name),
                            task,
                        )?;
                        statics
                            .db
                            .update_package(&old_package.to_owned(), new_package)?;
                        Ok(())
                    }
                },
                None => Ok(eprintln_no_valid_installer(&old_package.repo.name)),
            }
        }
        None => Ok(eprintln_no_installed_package_found(name)),
    }
}

pub fn list_packages(db: &PackageDBManager) -> () {
    let packages = db.fetch_all_packages();
    if packages.is_empty() {
        return println!("No packages installed");
    }
    let rows = packages
        .iter()
        .map(|p| {
            let path = p
                .install_info
                .executable_path
                .to_owned()
                .map(|p| display_path(&p).unwrap_or_default())
                .unwrap_or_default();
            vec![
                p.repo.name.to_owned(),
                p.version.to_owned(),
                path.to_owned(),
            ]
        })
        .collect();
    let column_headers = vec![
        "Name".to_owned(),
        "Version".to_owned(),
        "Installation Folder".to_owned(),
    ];
    println!("{}", generate_table_string(&column_headers, &rows));
}

// My magnum opus
pub fn generate_table_string(column_headers: &Vec<String>, rows: &Vec<Vec<String>>) -> String {
    let number_of_columns = column_headers.len();
    let number_of_rows = rows.len();
    // Calculate the maximum possible length of a string per column
    let max_length_per_column = (0..number_of_columns)
        .into_iter()
        .map(|column_idx| {
            (0..number_of_rows)
                .into_iter()
                .map(|row_idx: usize| rows[row_idx][column_idx].to_owned())
                .max_by_key(|item| item.len())
                .unwrap()
                .len()
        })
        .collect::<Vec<usize>>();
    // Update the obtained max lengths if any of the ones in the  column headers is longer
    let max_length_per_column = column_headers
        .iter()
        .zip(max_length_per_column.iter())
        .map(|(str, max_len)| str.len().max(max_len.to_owned()))
        .collect::<Vec<usize>>();
    // Format a row of data
    let last_idx = number_of_columns - 1;
    let format_row = |row: &Vec<String>| {
        row.iter()
            .enumerate()
            .map(|(idx, item)| {
                let max_len = max_length_per_column[idx]; // 4 spaces
                let delimeter = if idx == last_idx { "\n" } else { "    " };
                format!("{:<max_len$}{}", item, delimeter)
            })
            .collect::<String>()
    };

    let header_str = &format_row(&column_headers);
    let max_char_count_per_row =
        (4 * (number_of_columns - 1)) + max_length_per_column.iter().sum::<usize>();
    let seperator_str = "-".repeat(max_char_count_per_row);
    let data_str = rows.iter().map(|r| format_row(r)).collect::<String>();
    format!("{}{}\n{}", header_str, seperator_str, data_str)
}

pub async fn search_repos(query: &str, client: &Client) -> Result<(), KnownErrors> {
    let results = github::api::search(query, client).await?;
    if results.is_empty() {
        return Ok(println!("No results found."));
    }
    let rows = results
        .iter()
        .map(|r| {
            vec![
                r.full_name.to_owned(),
                r.description.to_owned().unwrap_or_default(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    let column_headers = vec!["Full Name".to_owned(), "Description".to_owned()];
    Ok(println!(
        "{}",
        generate_table_string(&column_headers, &rows)
    ))
}

pub fn exported_packages_filename() -> String {
    format!("{}-packages.txt", APP_NAME_LOWER)
}
pub fn export_packages(
    export_folder_path: &PathBuf,
    db: &PackageDBManager,
) -> Result<(), KnownErrors> {
    let mut final_str = "".to_owned();
    for p in db.fetch_all_packages() {
        let p_entry = format!("{}=={}\n", &p.repo.full_name, &p.version);
        final_str.push_str(&p_entry);
    }
    let export_file_path = export_folder_path.join(exported_packages_filename());
    let mut f = File::create(&export_file_path)?;
    f.write_all(final_str.as_bytes())?;
    Ok(println!("Exported at {}", display_path(&export_file_path)?))
}

pub async fn import_packages(
    export_file_path: &PathBuf,
    ignore_versions: bool,
    statics: &mut Statics,
) -> Result<(), KnownErrors> {
    let mut errored_packages: Vec<String> = Vec::new();
    for (name, version) in extract_package_name_and_version(export_file_path, ignore_versions)? {
        if let Err(err) = install_package(&name, &version, statics).await {
            errored_packages.push(name);
        }
    }
    match errored_packages.is_empty() {
        true => println!("Successfully imported all packages"),
        false => println!(
            "Errors encountered importing the following packages:\n{}",
            errored_packages.join(", ")
        ),
    };
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

pub fn run_package(name: &str, db: &PackageDBManager) -> Result<(), KnownErrors> {
    match db.find_package(name)? {
        Some(p) => match &p.install_info.executable_path {
            Some(ep) => {
                println!("Starting {}.. .", p.repo.name);
                Command::new(ep).spawn()?;
                Ok(())
            }
            None => Ok(eprintln!("No executable found for {}.", p.repo.name)),
        },
        None => Ok(eprintln_no_installed_package_found(name)),
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
        show_package("Senpwai", &db, client).await.unwrap();
        println!();
        db.add_package(senpwai_latest_package()).unwrap();
        show_package("SenZmaKi/Senpwai", &db, client).await.unwrap();
        println!();
        show_package("99419gb0", &db, client).await.unwrap();
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
