//!Exposes command endpoints

use crate::includes::{
    database::PackageDatabase,
    dist::Dist,
    dist::{DistType, InstallerDist, StartmenuFolders},
    error::SengetErrors,
    error::{
        check_for_other_errors, AlreadyUptoDateError, ExportFileNotFoundError,
        FailedToUninstallError, NoExecutableError, NoInstalledPackageError, NoPackageError,
        NoValidDistError, PackageAlreadyInstalledError, VersionAlreadyInstalledError,
    },
    github::{self, api::Repo},
    package::ExportedPackage,
    package::Package,
    utils::{loading_animation, setup_client, FolderItems, PathStr},
    utils::{DEBUG, IBYTES_TO_MBS_DIVISOR},
};
use regex::Regex;
use reqwest::Client;
use std::{
    fs::{self, DirEntry, File},
    io::{self, Write},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    u64,
};
use winreg::RegKey;
pub struct Statics {
    pub client: Client,
    pub version_regex: Regex,
    pub packages_folder_path: PathBuf,
    pub dists_folder_path: PathBuf,
    pub startmenu_folders: StartmenuFolders,
    pub user_uninstall_reg_key: RegKey,
    pub machine_uninstall_reg_key: RegKey,
}

impl Statics {
    pub fn new(root_dir: &Path) -> Result<Statics, SengetErrors> {
        let client = setup_client()?;
        let dists_folder_path = Dist::generate_dists_folder_path(root_dir)?;
        let startmenu_folders = InstallerDist::generate_startmenu_paths();
        let packages_folder_path =
            Dist::generate_packages_folder_path(root_dir, &startmenu_folders.appdata)?;
        let user_uninstall_reg_key = InstallerDist::generate_user_uninstall_reg_key()?;
        let machine_uninstall_reg_key = InstallerDist::generate_machine_uninstall_reg_key()?;
        let version_regex = github::api::Repo::generate_version_regex();
        Ok(Statics {
            client,
            version_regex,
            packages_folder_path,
            dists_folder_path,
            startmenu_folders,
            user_uninstall_reg_key,
            machine_uninstall_reg_key,
        })
    }
}

async fn find_repo(name: &str, client: &Client) -> Result<Option<Repo>, SengetErrors> {
    let name_lower = name.to_lowercase();
    let found_repo = github::api::search(name, client)
        .await?
        .into_iter()
        .find(|r| r.name.to_lowercase() == name_lower || r.full_name.to_lowercase() == name_lower);
    Ok(found_repo)
}

pub async fn show_package(
    name: &str,
    db: &PackageDatabase,
    client: &Client,
) -> Result<(), SengetErrors> {
    match db.find_package(name)? {
        Some(package) => Ok(println!("{}", package)),
        None => match find_repo(name, client).await? {
            Some(repo) => Ok(println!("{}", repo)),
            None => Err(NoPackageError.into()),
        },
    }
}

pub fn clear_cached_distributables(dists_folder_path: &Path) -> Result<(), SengetErrors> {
    let calc_size = |prev_size: u64, d: DirEntry| -> Result<u64, io::Error> {
        let p = d.path();
        if !p.is_file() {
            return Ok(prev_size);
        };
        let s = p.metadata()?.file_size();
        fs::remove_file(&p)?;
        Ok(prev_size + s)
    };
    let size = dists_folder_path
        .folder_items()?
        .into_iter()
        .try_fold(0, calc_size)?;
    println!("Cleared {} MBs", size / IBYTES_TO_MBS_DIVISOR);
    Ok(())
}
pub fn validate_cache_folder_size(dists_folder_path: &Path) -> Result<(), SengetErrors> {
    let size: u64 = dists_folder_path
        .folder_items()?
        .iter()
        .filter(|f| f.path().is_file())
        .flat_map(|f| f.metadata().map(|m| m.file_size()))
        .sum();
    let size_mbs = size / IBYTES_TO_MBS_DIVISOR;
    if size_mbs >= 100 && !DEBUG {
        println!(
            "Distributables cache folder is {} MBs, run \"senget clear-cache\" to clean it up",
            size_mbs
        );
    }
    Ok(())
}
pub fn purge_packages(db: &PackageDatabase) -> Result<(), SengetErrors> {
    let to_remove: Vec<Package> = db
        .fetch_all_packages()?
        .iter()
        .filter_map(|p| {
            if let Some(exe) = &p.install_info.executable_path {
                if !exe.is_file() {
                    return Some(p.clone());
                }
            };
            None
        })
        .collect();
    if to_remove.is_empty() {
        return Ok(println!("No packages to purge"));
    }
    for p in to_remove {
        db.remove_package(&p)?;
        println!("Purged {}", p.repo.name);
    }
    Ok(())
}

async fn update_all_packages(
    version: &str,
    db: &PackageDatabase,
    statics: &Statics,
) -> Result<(), SengetErrors> {
    let mut errored_packages: Vec<Vec<String>> = Vec::new();
    for p in db.fetch_all_packages()? {
        if let Err(err) = update_package(&p.repo.name, version, db, statics).await {
            match err {
                SengetErrors::AlreadyUptoDateError(_) => continue,
                SengetErrors::VersionAlreadyInstalledError(_) => continue,
                _ => errored_packages.push(vec![
                    p.repo.name,
                    format!("{:?}", check_for_other_errors(err)),
                ]),
            }
        }
    }
    match errored_packages.is_empty() {
        true => println!("Successfully updated all the necessary packages."),
        false => println!(
            "Errors encountered updating the following packages:\n{}",
            generate_table_string(
                &vec!["Name".to_owned(), "Error".to_owned()],
                &errored_packages
            )
        ),
    }
    Ok(())
}

pub async fn download_package(
    name: &str,
    version: &str,
    client: &Client,
    version_regex: &Regex,
    dists_folder_path: &Path,
    preferred_dist_type: &Option<DistType>,
) -> Result<(), SengetErrors> {
    let (_, _, dist_path) = internal_download_package(
        name,
        version,
        preferred_dist_type,
        client,
        version_regex,
        dists_folder_path,
    )
    .await?;
    println!("Downloaded at {}", dist_path.path_str()?);
    Ok(())
}
async fn internal_download_package(
    name: &str,
    version: &str,
    preferred_dist_type: &Option<DistType>,
    client: &Client,
    version_regex: &Regex,
    dists_folder_path: &Path,
) -> Result<(Repo, Dist, PathBuf), SengetErrors> {
    match find_repo(name, client).await? {
        Some(repo) => {
            let dist = match version {
                "latest" => {
                    repo.get_latest_dist(client, version_regex, preferred_dist_type)
                        .await?
                }
                version => {
                    repo.get_dist(client, version, version_regex, preferred_dist_type)
                        .await?
                }
            };
            match dist {
                Some(mut dist) => {
                    let dist_path = dist.download(client, dists_folder_path).await?;
                    if let Dist::Exe(exe_dist) = dist {
                        dist = exe_dist.check_if_is_actually_installer(&dist_path)?;
                    }
                    Ok((repo, dist, dist_path))
                }
                None => Err(NoValidDistError.into()),
            }
        }
        None => Err(NoPackageError.into()),
    }
}
pub async fn install_package(
    name: &str,
    version: &str,
    preferred_dist_type: &Option<DistType>,
    create_shortcut_file: bool,
    db: &PackageDatabase,
    statics: &Statics,
) -> Result<(), SengetErrors> {
    match db.find_package(name)? {
        Some(_) => Err(PackageAlreadyInstalledError.into()),
        None => {
            let (repo, dist, downloaded_package_path) = internal_download_package(
                name,
                version,
                preferred_dist_type,
                &statics.client,
                &statics.version_regex,
                &statics.dists_folder_path,
            )
            .await?;
            let task = || {
                dist.install(
                    &downloaded_package_path,
                    &statics.packages_folder_path,
                    create_shortcut_file,
                    &statics.startmenu_folders,
                    &statics.user_uninstall_reg_key,
                    &statics.machine_uninstall_reg_key,
                )
            };
            let install_info = loading_animation(format!("Installing {}.. .", repo.name), task)?;
            let package_name = repo.name.clone();
            let package = Package::new(dist.version().to_owned(), repo, install_info);
            db.add_package(package)?;
            println!("Successfully installed {}.", package_name);
            Ok(())
        }
    }
}

pub fn uninstall_package(
    name: &str,
    force: bool,
    startmenu_appdata_folder: &Path,
    db: &PackageDatabase,
) -> Result<(), SengetErrors> {
    match db.find_package(name)? {
        Some(package) => {
            let task = || -> Result<(), SengetErrors> {
                if !package.uninstall(startmenu_appdata_folder)? {
                    return Err(FailedToUninstallError.into());
                }
                Ok(())
            };
            let success =
                loading_animation(format!("Uninstalling {}", package.repo.name), task).is_ok();
            if !(success || force) {
                return Err(FailedToUninstallError.into());
            }
            db.remove_package(&package)?;
            if success {
                println!("Successfully uninstalled {}.", package.repo.name);
            } else {
                println!("Removed {} from package database.", package.repo.name);
            }
            Ok(())
        }
        None => Err(NoInstalledPackageError.into()),
    }
}

// FIXME: Fix updating into a different distributable e.g., from Exe to Installer
pub async fn update_handler(
    name: &str,
    version: &str,
    db: &PackageDatabase,
    statics: &Statics,
) -> Result<(), SengetErrors> {
    match name == "all" {
        true => update_all_packages("latest", db, statics).await,
        false => update_package(name, version, db, statics).await,
    }
}

async fn update_package(
    name: &str,
    version: &str,
    db: &PackageDatabase,
    statics: &Statics,
) -> Result<(), SengetErrors> {
    match db.find_package(name)? {
        Some(old_package) => {
            match old_package
                .get_dist(version, &statics.client, &statics.version_regex)
                .await?
            {
                Some(dist) => match old_package.version == dist.version() {
                    true => match version == "latest" {
                        true => Err(AlreadyUptoDateError.into()),
                        false => Err(VersionAlreadyInstalledError.into()),
                    },
                    false => {
                        println!(
                            "Updating {} from {} --> {}",
                            old_package.repo.name,
                            old_package.version,
                            dist.version()
                        );
                        let dist_path = dist
                            .download(&statics.client, &statics.dists_folder_path)
                            .await?;
                        let task = || {
                            old_package.install_updated_version(
                                dist,
                                &dist_path,
                                &statics.packages_folder_path,
                                &statics.startmenu_folders,
                                &statics.user_uninstall_reg_key,
                                &statics.machine_uninstall_reg_key,
                            )
                        };
                        let new_package = loading_animation(
                            format!("Updating {}.. .", old_package.repo.name),
                            task,
                        )?;
                        db.update_package(&old_package, new_package)?;
                        Ok(())
                    }
                },
                None => Err(NoValidDistError.into()),
            }
        }
        None => Err(NoInstalledPackageError.into()),
    }
}

pub fn list_packages(db: &PackageDatabase) -> Result<(), SengetErrors> {
    let packages = db.fetch_all_packages()?;
    let rows = packages
        .iter()
        .map(|p| {
            let path = p
                .install_info
                .installation_folder
                .as_ref()
                .map(|p| p.path_str().unwrap_or_default())
                .unwrap_or_default();
            vec![p.repo.name.clone(), p.version.clone(), path.clone()]
        })
        .collect();
    let column_headers = vec![
        "Name".to_owned(),
        "Version".to_owned(),
        "Installation Folder".to_owned(),
    ];
    println!("{}", generate_table_string(&column_headers, &rows));
    Ok(())
}

// My magnum opus
pub fn generate_table_string(column_headers: &Vec<String>, rows: &Vec<Vec<String>>) -> String {
    let number_of_columns = column_headers.len();
    let number_of_rows = rows.len();
    // Calculate the maximum possible length of a string per column
    let max_length_per_column = (0..number_of_columns)
        .map(|column_idx| {
            (0..number_of_rows)
                .map(|row_idx: usize| &rows[row_idx][column_idx])
                .max_by_key(|item| item.len())
                .unwrap()
                .len()
        })
        .collect::<Vec<usize>>();
    // Update the obtained max lengths if any of the ones in the  column headers is longer
    let max_length_per_column = column_headers
        .iter()
        .zip(max_length_per_column.iter())
        .map(|(str, max_len)| str.len().max(*max_len))
        .collect::<Vec<usize>>();
    // Format a row of data
    let last_idx = number_of_columns - 1;
    let format_row = |row: &Vec<String>| {
        row.iter()
            .enumerate()
            .fold(String::new(), |acc, (idx, item)| {
                let max_len = max_length_per_column[idx]; // 4 spaces
                let delimeter = if idx == last_idx { "\n" } else { "    " };
                format!("{}{:<max_len$}{}", acc, item, delimeter)
            })
    };

    let header_str = &format_row(column_headers);
    let max_char_count_per_row = (4 * (last_idx)) + max_length_per_column.iter().sum::<usize>();
    let seperator_str = "-".repeat(max_char_count_per_row);
    let data_str = rows.iter().map(format_row).collect::<String>();
    format!("{}{}\n{}", header_str, seperator_str, data_str)
}

pub async fn search_repos(query: &str, client: &Client) -> Result<(), SengetErrors> {
    let results = github::api::search(query, client).await?;
    if results.is_empty() {
        return Ok(println!("No results found"));
    }
    let rows = results
        .iter()
        .map(|r| {
            vec![
                r.full_name.clone(),
                r.description.clone().unwrap_or_default(),
            ]
        })
        .collect();
    let column_headers = vec!["Full Name".to_owned(), "Description".to_owned()];
    Ok(println!(
        "{}",
        generate_table_string(&column_headers, &rows)
    ))
}

pub fn export_packages(export_file_path: &Path, db: &PackageDatabase) -> Result<(), SengetErrors> {
    let exported_packages: Vec<ExportedPackage> =
        db.fetch_all_packages()?
            .iter()
            .fold(Vec::new(), |mut prev, p| {
                prev.push(p.export());
                prev
            });
    let json_string = serde_json::to_string_pretty(&exported_packages)?;
    File::create(export_file_path)?.write_all(json_string.as_bytes())?;
    Ok(println!("Exported at {}", export_file_path.path_str()?))
}

pub async fn import_packages(
    export_file_path: &PathBuf,
    ignore_versions: bool,
    db: &PackageDatabase,
    statics: &Statics,
) -> Result<(), SengetErrors> {
    let mut errored_packages: Vec<Vec<String>> = Vec::new();
    if !export_file_path.is_file() {
        return Err(ExportFileNotFoundError.into());
    }
    let packages_str = fs::read_to_string(export_file_path)?;
    let packages: Vec<ExportedPackage> = serde_json::from_str(&packages_str)?;
    for p in packages {
        let version = if ignore_versions {
            "latest"
        } else {
            &p.version
        };
        if let Err(err) = install_package(
            &p.full_name,
            version,
            &Some(p.preferred_dist_type),
            p.create_shortcut_file,
            db,
            statics,
        )
        .await
        {
            match err {
                SengetErrors::PackageAlreadyInstalledError(_) => continue,
                _ => errored_packages.push(vec![
                    p.full_name,
                    format!("{:?}", check_for_other_errors(err)),
                ]),
            }
        }
    }
    match errored_packages.is_empty() {
        true => println!("Successfully imported all the necessary packages."),
        false => println!(
            "Errors encountered importing the following packages:\n{}",
            generate_table_string(
                &vec!["Name".to_owned(), "Error".to_owned()],
                &errored_packages
            )
        ),
    };
    Ok(())
}

pub fn run_package(
    name: &str,
    no_wait: bool,
    args: &Vec<&String>,
    db: &PackageDatabase,
) -> Result<(), SengetErrors> {
    match db.find_package(name)? {
        Some(p) => match &p.install_info.executable_path {
            Some(ep) => {
                if !ep.is_file() {
                    return Err(NoExecutableError.into());
                }
                println!("Starting {}.. .", p.repo.name);
                let mut command = Command::new(ep);
                command.args(args);
                if no_wait {
                    command.spawn()?;
                } else {
                    command.status()?;
                };
                Ok(())
            }
            None => Err(NoExecutableError.into()),
        },
        None => Err(NoInstalledPackageError.into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::includes::{
        commands::{list_packages, search_repos, show_package},
        test_utils::{client, db_manager, hatt_package, senpwai_latest_package},
    };

    #[tokio::test]
    async fn test_show_package() {
        let db = db_manager();
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
        let db = db_manager();
        db.add_package(senpwai_latest_package()).unwrap();
        db.add_package(hatt_package()).unwrap();
        list_packages(&db).unwrap();
    }
}
