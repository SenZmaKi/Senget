//!Manages package download and installation

use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressStyle};
use lnk::ShellLink;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env,
    fs::{self, DirEntry, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey,
};
use zip::ZipArchive;

use crate::includes::{
    error::{ContentLengthError, RequestIoContentLengthError},
    utils::{display_path, DEBUG, MSI_EXEC},
};

use crate::includes::error::{NoExeFoundError, ZipIoExeError};

// Running an msi installer that needs admin access silently is problematic since
// it'll just exit silently without an error if it fails cause of lack of admin access
// and there's no way to know that it needs admin access ahead of time
// const MSI_SILENT_ARG: &str = "/qn";
const INNO_SILENT_ARG: &str = "/VERYSILENT";
const NSIS_SILENT_ARG: &str = "/S";
const STARTMENU_FOLDER_ENDPOINT: &str = "\\Microsoft\\Windows\\Start Menu\\Programs";

#[derive(ValueEnum, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DistType {
    Installer,
    Zip,
    Exe,
}
impl From<clap::builder::Str> for DistType {
    fn from(value: clap::builder::Str) -> Self {
        if value == "installer" {
            return Self::Installer;
        }
        if value == "zip" {
            return Self::Zip;
        }
        Self::Exe
    }
}

/// The type of the distributable
#[derive(Debug, Clone)]
pub enum Dist {
    /// Standalone executable distributable
    Exe(ExeDist),
    /// Zipped package distributable
    Zip(ZipDist),
    /// Installer distributable e.g., inno-setup, nsis-installer or msi
    Installer(InstallerDist),
}

impl Dist {
    pub fn version(&self) -> &str {
        match self {
            Dist::Exe(dist) => &dist.package_info.version,
            Dist::Zip(dist) => &dist.package_info.version,
            Dist::Installer(dist) => &dist.package_info.version,
        }
    }
    pub async fn download(
        &self,
        client: &Client,
        packages_folder_path: &Path,
        dists_folder_path: &Path,
    ) -> Result<PathBuf, RequestIoContentLengthError> {
        match self {
            Dist::Exe(dist) => dist.download(packages_folder_path, client).await,
            Dist::Zip(dist) => dist.download(dists_folder_path, client).await,
            Dist::Installer(dist) => dist.download(dists_folder_path, client).await,
        }
    }

    pub fn install(
        &self,
        downloaded_package_path: PathBuf,
        packages_folder_path: &Path,
        startmenu_folders: &(PathBuf, PathBuf),
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<InstallInfo, ZipIoExeError> {
        match self {
            Dist::Exe(_) => Ok(ExeDist::install(downloaded_package_path)),
            Dist::Zip(dist) => dist.install(packages_folder_path, &downloaded_package_path),
            Dist::Installer(dist) => Ok(dist.install(
                &downloaded_package_path,
                startmenu_folders,
                user_uninstall_reg_key,
                machine_uninstall_reg_key,
            )?),
        }
    }

    fn generate_path_from_root(name: &str, root_dir: &Path) -> Result<PathBuf, io::Error> {
        let path = root_dir.join(name);
        if !path.is_dir() {
            fs::create_dir(&path)?;
        }
        Ok(path)
    }

    pub fn generate_dists_folder_path(root_dir: &Path) -> Result<PathBuf, io::Error> {
        Self::generate_path_from_root("Package-Installers", root_dir)
    }

    pub fn generate_packages_folder_path(root_dir: &Path) -> Result<PathBuf, io::Error> {
        Self::generate_path_from_root("Packages", root_dir)
    }
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    name: String,
    file_title: String,
    pub download_url: String,
    pub version: String,
}

impl PackageInfo {
    pub fn fetch_dist(self, dist_type: DistType) -> Dist {
        match dist_type {
            DistType::Exe => Dist::Exe(ExeDist { package_info: self }),
            DistType::Zip => Dist::Zip(ZipDist { package_info: self }),
            DistType::Installer => Dist::Installer(InstallerDist { package_info: self }),
        }
    }
    pub fn new(name: String, download_url: String, version: String, file_title: String) -> Self {
        Self {
            name,
            download_url,
            version,
            file_title,
        }
    }
    pub async fn download(
        &self,
        download_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, RequestIoContentLengthError> {
        let path = download_folder_path.join(&self.file_title);
        let mut file = File::create(&path)?;
        let mut response = client.get(&self.download_url).send().await?;
        let progress_bar = ProgressBar::new(response.content_length().ok_or(ContentLengthError)?);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {bytes}/{total_bytes} ({eta} left)")
                .expect("Valid template"),
        );
        let mut progress = 0;
        progress_bar.set_position(progress);
        progress_bar.set_message(format!("Downloading {}", self.file_title));
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk)?;
            progress += chunk.len() as u64;
            progress_bar.set_position(progress);
        }
        progress_bar.finish_with_message("Download complete");
        Ok(path)
    }
}

#[derive(Debug, Clone)]
pub struct ExeDist {
    pub package_info: PackageInfo,
}

impl ExeDist {
    pub async fn download(
        &self,
        packages_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, RequestIoContentLengthError> {
        let package_folder = packages_folder_path.join(&self.package_info.name);
        if !package_folder.is_dir() {
            fs::create_dir(&package_folder)?;
        }
        self.package_info.download(&package_folder, client).await
    }

    pub fn install(downloaded_package_path: PathBuf) -> InstallInfo {
        let installation_folder = Some(downloaded_package_path.parent().unwrap().to_owned());
        let executable_path = Some(downloaded_package_path);
        InstallInfo {
            executable_path,
            installation_folder,
            uninstall_command: None,
            dist_type: DistType::Exe,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ZipDist {
    pub package_info: PackageInfo,
}

impl ZipDist {
    pub async fn download(
        &self,
        dists_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, RequestIoContentLengthError> {
        self.package_info.download(dists_folder_path, client).await
    }
    fn find_executable_path(
        &self,
        extracted_folder_items: Vec<DirEntry>,
    ) -> Result<Option<PathBuf>, io::Error> {
        let self_lower_name = self.package_info.name.to_lowercase();
        let found_exe = extracted_folder_items.into_iter().find_map(|de| {
            let lower_file_name = de.file_name().to_str().unwrap_or_default().to_lowercase();
            if lower_file_name.ends_with("exe") && lower_file_name.contains(&self_lower_name) {
                return Some(de.path());
            }
            None
        });
        Ok(found_exe)
    }
    pub fn install(
        &self,
        packages_folder_path: &Path,
        downloaded_package_path: &Path,
    ) -> Result<InstallInfo, ZipIoExeError> {
        let package_folder = packages_folder_path.join(&self.package_info.name);
        ZipArchive::new(File::open(downloaded_package_path)?)?.extract(&package_folder)?;
        let extracted_folder_items = fs::read_dir(packages_folder_path)?.try_fold(
            Vec::new(),
            |mut ext_fold_items, de| -> Result<Vec<DirEntry>, io::Error> {
                ext_fold_items.push(de?);
                Ok(ext_fold_items)
            },
        )?;
        if extracted_folder_items.len() == 1 {
            let path = extracted_folder_items[0].path();
            if path.is_dir() {
                fs::rename(path, &package_folder)?;
            }
        }
        let executable_path = self.find_executable_path(extracted_folder_items)?;
        if executable_path.is_none() {
            return Err(ZipIoExeError::NoExeFouundError(NoExeFoundError));
        }
        if !DEBUG {
            fs::remove_file(downloaded_package_path)?;
        }
        Ok(InstallInfo {
            executable_path,
            installation_folder: Some(package_folder),
            uninstall_command: None,
            dist_type: DistType::Zip,
        })
    }
}

#[derive(Debug, Clone)]
pub struct InstallerDist {
    pub package_info: PackageInfo,
}
impl InstallerDist {
    const UNINSTALL_KEY_STR: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    pub async fn download(
        &self,
        dists_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, RequestIoContentLengthError> {
        let prev_installer = dists_folder_path.join(&self.package_info.file_title);
        if prev_installer.is_file() {
            println!(
                "Using cached package at: {}",
                display_path(&prev_installer).unwrap_or_default()
            );
            return Ok(prev_installer);
        }
        self.package_info.download(dists_folder_path, client).await
    }

    pub fn generate_machine_uninstall_reg_key() -> Result<RegKey, io::Error> {
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(InstallerDist::UNINSTALL_KEY_STR)
    }

    pub fn generate_user_uninstall_reg_key() -> Result<RegKey, io::Error> {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(InstallerDist::UNINSTALL_KEY_STR)
    }

    pub fn generate_startmenu_paths() -> (PathBuf, PathBuf) {
        let gen =
            |envvar: &str| PathBuf::from(env::var(envvar).unwrap() + STARTMENU_FOLDER_ENDPOINT);
        (gen("APPDATA"), gen("PROGRAMDATA"))
    }

    fn fetch_shortcut_files(
        files: &mut HashSet<PathBuf>,
        startmenu_folder: &Path,
        check_inner_folders: bool,
    ) -> Result<(), io::Error> {
        for e in startmenu_folder.read_dir()? {
            match e {
                Ok(e) => {
                    let e = e.path();
                    if e.is_file() && e.ends_with(".lnk") {
                        files.insert(e);
                    } else if check_inner_folders && e.is_dir() {
                        InstallerDist::fetch_shortcut_files(files, startmenu_folder, false)?;
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    fn fetch_reg_keys(parent_regkey: &RegKey) -> Result<HashSet<String>, io::Error> {
        parent_regkey.enum_keys().collect()
    }

    fn run_installation(file_extension: &str, file_path: &Path) -> Result<(), std::io::Error> {
        match file_extension == "msi" {
            true => Command::new(MSI_EXEC).arg("/i").arg(file_path).output()?,
            false => Command::new(file_path)
                .args([INNO_SILENT_ARG, NSIS_SILENT_ARG])
                .output()?,
        };
        Ok(())
    }

    fn statically_generate_package_shortcut(&self, startmenu_folder: &Path) -> Option<PathBuf> {
        let shortcut_file_name = format!("{}.lnk", &self.package_info.name);
        let shortcut_path = startmenu_folder.join(&shortcut_file_name);
        if shortcut_path.is_file() {
            return Some(shortcut_path);
        }
        let shortcut_path = startmenu_folder
            .join(&self.package_info.name)
            .join(shortcut_file_name);
        if shortcut_path.is_file() {
            return Some(shortcut_path);
        }
        None
    }

    fn find_shortcut_target(shortcut_path: &Path) -> Option<PathBuf> {
        let lnk = ShellLink::open(shortcut_path).ok()?;
        let target = lnk.link_info().as_ref()?.local_base_path().as_ref()?;
        Some(PathBuf::from(target))
    }

    fn dynamically_find_package_shortcut(
        target_name_lower: &str,
        shortcut_files_before: &HashSet<PathBuf>,
        startmenu_folder: &Path,
    ) -> Result<Option<PathBuf>, io::Error> {
        let mut shortcut_files_after = HashSet::<PathBuf>::new();
        InstallerDist::fetch_shortcut_files(&mut shortcut_files_after, startmenu_folder, true)?;

        let found_shortcut = shortcut_files_after
            .difference(shortcut_files_before)
            .find(|s| {
                s.file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    .contains(target_name_lower)
            })
            .cloned();
        Ok(found_shortcut)
    }

    fn extract_uninstall_command_from_keys(
        target_name_lower: &str,
        new_keys: Vec<&String>,
        parent_regkey: &RegKey,
    ) -> Result<Option<String>, io::Error> {
        for k in new_keys {
            let subkey = parent_regkey.open_subkey(k)?;
            let disp_name: Result<String, io::Error> = subkey.get_value("DisplayName");
            if let Ok(disp_name) = disp_name {
                if disp_name.to_lowercase().contains(target_name_lower) {
                    let uninstall_command = subkey
                        .get_value("QuietUninstallString")
                        .or_else(|_| subkey.get_value("UninstallString"))
                        .ok();
                    return Ok(uninstall_command);
                }
            }
        }
        Ok(None)
    }
    fn fetch_uninstall_command_for_key(
        target_name_lower: &str,
        after_keys: &HashSet<String>,
        before_keys: &HashSet<String>,
        parent_regkey: &RegKey,
    ) -> Result<Option<String>, io::Error> {
        let new_keys = after_keys.difference(before_keys).collect::<Vec<&String>>();
        InstallerDist::extract_uninstall_command_from_keys(
            target_name_lower,
            new_keys,
            parent_regkey,
        )
    }

    fn fetch_uninstall_command(
        target_name_lower: &str,
        installation_folder: &Option<PathBuf>,
        user_reg_keys_before: &HashSet<String>,
        machine_reg_keys_before: &HashSet<String>,
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<Option<String>, io::Error> {
        let user_reg_keys_after = InstallerDist::fetch_reg_keys(user_uninstall_reg_key)?;
        let mut uninstall_command = InstallerDist::fetch_uninstall_command_for_key(
            target_name_lower,
            &user_reg_keys_after,
            user_reg_keys_before,
            user_uninstall_reg_key,
        )?;
        if uninstall_command.is_none() {
            let machine_reg_keys_after = InstallerDist::fetch_reg_keys(machine_uninstall_reg_key)?;
            uninstall_command = InstallerDist::fetch_uninstall_command_for_key(
                target_name_lower,
                &machine_reg_keys_after,
                machine_reg_keys_before,
                machine_uninstall_reg_key,
            )?;
        }
        if uninstall_command.is_none() && installation_folder.is_some() {
            uninstall_command = InstallerDist::fetch_uninstall_command_from_executable(
                installation_folder.as_ref().expect("is_some"),
            )?;
        }
        Ok(uninstall_command)
    }
    pub fn fetch_uninstall_command_from_executable(
        installation_folder: &Path,
    ) -> Result<Option<String>, io::Error> {
        for e in installation_folder.read_dir()?.flatten() {
            let file_name = e.file_name().to_str().unwrap().to_lowercase();
            if file_name.contains("unins") && file_name.ends_with(".exe") {
                return Ok(Some(display_path(&e.path())?));
            }
        }
        Ok(None)
    }

    pub fn install(
        &self,
        installer_path: &Path,
        startmenu_folders: &(PathBuf, PathBuf),
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<InstallInfo, io::Error> {
        let user_reg_keys_before = InstallerDist::fetch_reg_keys(user_uninstall_reg_key)?;
        let machine_reg_keys_before = InstallerDist::fetch_reg_keys(machine_uninstall_reg_key)?;
        let mut shortcut_files_before = HashSet::<PathBuf>::new();
        InstallerDist::fetch_shortcut_files(
            &mut shortcut_files_before,
            &startmenu_folders.0,
            true,
        )?;
        InstallerDist::fetch_shortcut_files(
            &mut shortcut_files_before,
            &startmenu_folders.1,
            true,
        )?;
        let file_extension = installer_path
            .extension()
            .unwrap()
            .to_str()
            .unwrap_or_default();
        InstallerDist::run_installation(file_extension, installer_path)?;
        if !DEBUG {
            fs::remove_file(installer_path)?;
        }

        let self_name_lower = self.package_info.name.to_lowercase();
        let mut executable_path = self
            .statically_generate_package_shortcut(&startmenu_folders.0)
            .or_else(|| self.statically_generate_package_shortcut(&startmenu_folders.1));
        if executable_path.is_none() {
            executable_path = InstallerDist::dynamically_find_package_shortcut(
                &self_name_lower,
                &shortcut_files_before,
                &startmenu_folders.0,
            )?
        };
        if executable_path.is_none() {
            executable_path = InstallerDist::dynamically_find_package_shortcut(
                &self_name_lower,
                &shortcut_files_before,
                &startmenu_folders.1,
            )?;
        };
        executable_path
            .as_ref()
            .and_then(|path| InstallerDist::find_shortcut_target(path));
        let installation_folder = executable_path
            .as_ref()
            .and_then(|ep| ep.parent().map(PathBuf::from));

        let uninstall_command = InstallerDist::fetch_uninstall_command(
            &self_name_lower,
            &installation_folder,
            &user_reg_keys_before,
            &machine_reg_keys_before,
            user_uninstall_reg_key,
            machine_uninstall_reg_key,
        )?;

        Ok(InstallInfo {
            executable_path,
            installation_folder,
            uninstall_command,
            dist_type: DistType::Installer,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallInfo {
    pub executable_path: Option<PathBuf>,
    pub installation_folder: Option<PathBuf>,
    pub uninstall_command: Option<String>,
    pub dist_type: DistType,
}

#[cfg(test)]
mod tests {

    use crate::includes::{
        dist::InstallerDist,
        test_utils::{client, package_dist_dir, senpwai_latest_dist},
    };

    #[tokio::test]
    async fn test_downloading_dist() {
        let f_path = senpwai_latest_dist()
            .download(&package_dist_dir(), &client())
            .await
            .expect("Downloading");
        assert!(f_path.is_file());
    }

    #[test]
    fn test_installer_installation() {
        let path = package_dist_dir().join("Senpwai-Installer.exe");
        let install_info = senpwai_latest_dist()
            .install(
                &path,
                &InstallerDist::generate_startmenu_paths(),
                &InstallerDist::generate_user_uninstall_reg_key()
                    .expect("Ok(user_uninstall_reg_key)"),
                &InstallerDist::generate_machine_uninstall_reg_key()
                    .expect("Ok(machine_uninstall_reg_key)"),
            )
            .expect("Some(install_info)");
        println!("Results for test_normal_installation\n {:?}", install_info);

        assert!(install_info
            .executable_path
            .expect("Some(executable_path)")
            .is_file());
        assert!(install_info.uninstall_command.is_some());
    }
}

