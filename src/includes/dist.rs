//!Manages package download and installation

use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressStyle};
use lnk;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::{self, Read};
use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    process::Command,
};
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey,
};
use zip::ZipArchive;

use crate::includes::package::MSI_EXEC;
use crate::includes::utils::Cmd;
use crate::includes::{
    error::{NoExeFoundInZipError, SengetErrors},
    senget_manager::env::add_package_folder_to_senget_env_var,
    utils::{FilenameLower, FolderItems, MoveDirAll, PathStr, Take, DEBUG},
};

// Running an msi installer that needs admin access silently is problematic since
// it'll just exit silently without an error if it fails cause of lack of admin access
// and there's no way to know that it needs admin access ahead of time
// const MSI_SILENT_ARG: &str = "/qn";
const INNO_SILENT_ARG: &str = "/VERYSILENT";
const NSIS_SILENT_ARG: &str = "/S";
const STARTMENU_FOLDER_ENDPOINT: &str = "\\Microsoft\\Windows\\Start Menu\\Programs";
const PROGRAMS_FOLDER: &str = "Local\\Programs";

pub struct StartmenuFolders {
    pub appdata: PathBuf,
    pub programdata: PathBuf,
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum Dist {
    /// Zipped package distributable
    Zip(ZipDist),
    /// Standalone executable distributable
    Exe(ExeDist),
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
        dists_folder_path: &Path,
    ) -> Result<PathBuf, SengetErrors> {
        match self {
            Dist::Exe(dist) => dist.download(dists_folder_path, client).await,
            Dist::Zip(dist) => dist.download(dists_folder_path, client).await,
            Dist::Installer(dist) => dist.download(dists_folder_path, client).await,
        }
    }

    fn create_shortcut_file(
        package_name: &str,
        executable_path: &Path,
        appdata_startmenu_folder: &Path,
    ) -> Result<(), mslnk::MSLinkError> {
        // For whatever reason mslnk doesn't work with a normal Path struct, only a String
        let lnk = mslnk::ShellLink::new(executable_path.path_str()?)?;
        let lnk_path = appdata_startmenu_folder.join(format!("{}.lnk", package_name));
        if lnk_path.is_file() {
            fs::remove_file(&lnk_path)?
        }
        lnk.create_lnk(lnk_path)
    }
    fn package_info(&self) -> &PackageInfo {
        match self {
            Dist::Exe(dist) => &dist.package_info,
            Dist::Zip(dist) => &dist.package_info,
            Dist::Installer(dist) => &dist.package_info,
        }
    }

    pub fn install(
        &self,
        downloaded_dist_path: &Path,
        packages_folder_path: &Path,
        create_shortcut_file: bool,
        startmenu_folders: &StartmenuFolders,
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<InstallInfo, SengetErrors> {
        let install_info = match self {
            Dist::Exe(dist) => dist.install(
                downloaded_dist_path,
                packages_folder_path,
                create_shortcut_file,
            )?,
            Dist::Zip(dist) => dist.install(
                downloaded_dist_path,
                packages_folder_path,
                create_shortcut_file,
            )?,
            Dist::Installer(dist) => dist.install(
                downloaded_dist_path,
                create_shortcut_file,
                startmenu_folders,
                user_uninstall_reg_key,
                machine_uninstall_reg_key,
            )?,
        };
        if !matches!(self, Dist::Installer(_)) && create_shortcut_file {
            Dist::create_shortcut_file(
                &self.package_info().name,
                install_info.executable_path.as_ref().unwrap(),
                &startmenu_folders.appdata,
            )?;
        }
        if let Some(installation_folder) = install_info.installation_folder.as_ref() {
            add_package_folder_to_senget_env_var(
                &installation_folder.path_str().unwrap_or_default(),
            )?;
        }
        Ok(install_info)
    }

    fn generate_path_from_root(name: &str, root_dir: &Path) -> Result<PathBuf, io::Error> {
        let path = root_dir.join(name);
        if !path.is_dir() {
            fs::create_dir(&path)?;
        }
        Ok(path)
    }

    pub fn generate_dists_folder_path(root_dir: &Path) -> Result<PathBuf, io::Error> {
        Self::generate_path_from_root("distributables", root_dir)
    }

    pub fn generate_packages_folder_path(
        root_dir: &Path,
        appdata_startmenu_folder: &Path,
    ) -> Result<PathBuf, io::Error> {
        if DEBUG {
            return Self::generate_path_from_root("packages", root_dir);
        }
        Ok(appdata_startmenu_folder.join(PROGRAMS_FOLDER))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackageInfo {
    name: String,
    file_title: String,
    file_size: u64,
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
    pub fn new(
        name: String,
        download_url: String,
        version: String,
        file_title: String,
        file_size: u64,
    ) -> Self {
        Self {
            name,
            download_url,
            version,
            file_title,
            file_size,
        }
    }

    pub async fn download(
        &self,
        download_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, SengetErrors> {
        let path = download_folder_path.join(&self.file_title);
        let mut file = File::create(&path)?;
        let mut response = client.get(&self.download_url).send().await?;
        let progress_bar = ProgressBar::new(self.file_size);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.green/orange}] {bytes}/{total_bytes} ({eta} left)")
                .unwrap()
                .progress_chars("#|-"),
        );
        let mut progress = 0;
        progress_bar.set_position(progress);
        progress_bar.set_message(format!("Downloading {}:", self.file_title));
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk)?;
            progress += chunk.len() as u64;
            progress_bar.set_position(progress);
        }
        progress_bar.finish_and_clear();
        Ok(path)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExeDist {
    pub package_info: PackageInfo,
}

impl ExeDist {
    pub async fn download(
        &self,
        distributables_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, SengetErrors> {
        self.package_info
            .download(distributables_folder_path, client)
            .await
    }

    // NOTE: Make sure you run this every time a Dist is downloaded
    pub fn check_if_is_actually_installer(
        self,
        downloaded_dist_path: &Path,
    ) -> Result<Dist, SengetErrors> {
        let mut buffer = Vec::new();
        File::open(downloaded_dist_path)?.read_to_end(&mut buffer)?;
        let text: String = buffer
            .into_iter()
            .filter_map(|byte| {
                if byte.is_ascii_alphabetic() {
                    return Some(byte as char);
                }
                None
            })
            .collect();
        if text.contains("Inno") || text.contains("Nullsoft") {
            return Ok(Dist::Installer(InstallerDist {
                package_info: self.package_info,
            }));
        }

        Ok(Dist::Exe(self))
    }

    pub fn install(
        &self,
        downloaded_dist_path: &Path,
        packages_folder_path: &Path,
        create_shortcut_file: bool,
    ) -> Result<InstallInfo, io::Error> {
        let p_folder_path = packages_folder_path.join(&self.package_info.name);
        if !p_folder_path.is_dir() {
            fs::create_dir(&p_folder_path)?;
        };
        let exe_path = p_folder_path.join(format!("{}.exe", self.package_info.name));
        if DEBUG {
            fs::copy(downloaded_dist_path, &exe_path)?;
        } else {
            fs::rename(downloaded_dist_path, &exe_path)?;
        }
        let installation_folder = Some(p_folder_path);
        let executable_path = Some(exe_path);
        let install_info = InstallInfo {
            executable_path,
            installation_folder,
            uninstall_command: None,
            dist_type: DistType::Exe,
            create_shortcut_file,
        };
        Ok(install_info)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZipDist {
    pub package_info: PackageInfo,
}

impl ZipDist {
    pub async fn download(
        &self,
        dists_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, SengetErrors> {
        if DEBUG {
            let path = dists_folder_path.join(&self.package_info.file_title);
            if path.is_file() {
                return Ok(path);
            }
        }
        self.package_info.download(dists_folder_path, client).await
    }

    fn find_executable_path(
        self_name_lower: &str,
        folder: PathBuf,
    ) -> Result<Option<PathBuf>, io::Error> {
        let mut queue = VecDeque::new();
        let self_exe_name_lower = format!("{}.exe", self_name_lower);
        queue.push_back(folder);
        let mut found_exe: Option<PathBuf> = None;
        // Looks for the executable breadth first
        while let Some(current_folder) = queue.pop_front() {
            let folder_items: Vec<PathBuf> = current_folder
                .folder_items()?
                .into_iter()
                .map(|item| item.path())
                .collect();
            for item in folder_items.iter() {
                let lower_file_name = item.filename_lower();
                if lower_file_name == self_exe_name_lower {
                    return Ok(Some(item.clone()));
                }
                if found_exe.is_none()
                    && lower_file_name.ends_with("exe")
                    && lower_file_name.contains(self_name_lower)
                {
                    found_exe = Some(item.clone());
                }
            }

            folder_items
                .into_iter()
                .filter(|f| f.is_dir())
                .for_each(|f| queue.push_back(f));
        }
        Ok(found_exe)
    }

    fn find_inner_unzip_folder(outer_unzip_folder: PathBuf) -> Result<PathBuf, io::Error> {
        let inner_folders: Vec<PathBuf> = outer_unzip_folder
            .folder_items()?
            .into_iter()
            .filter_map(|item| {
                let path = item.path();
                if path.is_dir() {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        // != 1 instead of > 1 so that if the folder is empty we dont get array bounds error at folder_items[0]
        if inner_folders.len() != 1 {
            return Ok(outer_unzip_folder);
        }
        ZipDist::find_inner_unzip_folder(inner_folders.take(0).unwrap())
    }
    pub fn install(
        &self,
        downloaded_dist_path: &Path,
        packages_folder_path: &Path,
        create_shortcut_file: bool,
    ) -> Result<InstallInfo, SengetErrors> {
        let installation_folder = packages_folder_path.join(&self.package_info.name);
        ZipArchive::new(File::open(downloaded_dist_path)?)?.extract(&installation_folder)?;
        let inner_unzip_dir = ZipDist::find_inner_unzip_folder(installation_folder.to_owned())?;
        if inner_unzip_dir != installation_folder {
            inner_unzip_dir.move_dir_all(&installation_folder)?;
        }
        if !DEBUG {
            fs::remove_file(downloaded_dist_path)?;
        }
        let self_name_lower = self.package_info.name.to_lowercase();
        let executable_path =
            ZipDist::find_executable_path(&self_name_lower, installation_folder.to_owned())?;
        if executable_path.is_none() {
            fs::remove_dir_all(installation_folder)?;
            return Err(SengetErrors::NoExeFound(NoExeFoundInZipError));
        }
        Ok(InstallInfo {
            executable_path,
            installation_folder: Some(installation_folder),
            uninstall_command: None,
            dist_type: DistType::Zip,
            create_shortcut_file,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallerDist {
    pub package_info: PackageInfo,
}
impl InstallerDist {
    const UNINSTALL_KEY_STR: &'static str =
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    pub async fn download(
        &self,
        dists_folder_path: &Path,
        client: &reqwest::Client,
    ) -> Result<PathBuf, SengetErrors> {
        let prev_installer = dists_folder_path.join(&self.package_info.file_title);
        if prev_installer.is_file() {
            println!(
                "Using cached distributable at: {}",
                prev_installer.path_str()?
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

    pub fn generate_startmenu_paths() -> StartmenuFolders {
        let gen =
            |envvar: &str| PathBuf::from(env::var(envvar).unwrap() + STARTMENU_FOLDER_ENDPOINT);
        let appdata = gen("APPDATA");
        let programdata = gen("PROGRAMDATA");
        StartmenuFolders {
            appdata,
            programdata,
        }
    }

    fn fetch_shortcut_files(
        files: &mut HashSet<PathBuf>,
        startmenu_folder: &Path,
    ) -> Result<(), io::Error> {
        for e in startmenu_folder.folder_items()? {
            let e = e.path();
            if e.is_file() && e.ends_with(".lnk") {
                files.insert(e);
            } else if e.is_dir() {
                InstallerDist::fetch_shortcut_files(files, &e)?;
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
            false => Command::cmd()
                .arg(file_path)
                .arg(INNO_SILENT_ARG)
                .arg(NSIS_SILENT_ARG)
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
        let lnk = lnk::ShellLink::open(shortcut_path).ok()?;
        let target = lnk.link_info().as_ref()?.local_base_path().as_ref()?;
        Some(PathBuf::from(target))
    }

    fn dynamically_find_package_shortcut(
        target_name_lower: &str,
        shortcut_files_before: &HashSet<PathBuf>,
        startmenu_folder: &Path,
    ) -> Result<Option<PathBuf>, io::Error> {
        let mut shortcut_files_after = HashSet::<PathBuf>::new();
        InstallerDist::fetch_shortcut_files(&mut shortcut_files_after, startmenu_folder)?;

        let found_shortcut = shortcut_files_after
            .difference(shortcut_files_before)
            .find(|s| s.filename_lower().contains(target_name_lower))
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
        for e in installation_folder.folder_items()?.iter() {
            let e_path = e.path();
            let file_name_lower = e_path.filename_lower();
            if file_name_lower.contains("unins") && file_name_lower.ends_with(".exe") {
                let some_path = Some(e_path.path_str()?);
                return Ok(some_path);
            }
        }
        Ok(None)
    }

    pub fn install(
        &self,
        installer_path: &Path,
        create_shortcut_file: bool,
        startmenu_folders: &StartmenuFolders,
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<InstallInfo, io::Error> {
        let user_reg_keys_before = InstallerDist::fetch_reg_keys(user_uninstall_reg_key)?;
        let machine_reg_keys_before = InstallerDist::fetch_reg_keys(machine_uninstall_reg_key)?;
        let mut shortcut_files_before = HashSet::<PathBuf>::new();
        InstallerDist::fetch_shortcut_files(
            &mut shortcut_files_before,
            &startmenu_folders.appdata,
        )?;
        InstallerDist::fetch_shortcut_files(
            &mut shortcut_files_before,
            &startmenu_folders.programdata,
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

        let mut shortcut_path = self
            .statically_generate_package_shortcut(&startmenu_folders.appdata)
            .or_else(|| self.statically_generate_package_shortcut(&startmenu_folders.programdata));
        let self_name_lower = self.package_info.name.to_lowercase();
        if shortcut_path.is_none() {
            shortcut_path = InstallerDist::dynamically_find_package_shortcut(
                &self_name_lower,
                &shortcut_files_before,
                &startmenu_folders.appdata,
            )?
        };
        if shortcut_path.is_none() {
            shortcut_path = InstallerDist::dynamically_find_package_shortcut(
                &self_name_lower,
                &shortcut_files_before,
                &startmenu_folders.programdata,
            )?;
        };
        let executable_path = shortcut_path
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
            create_shortcut_file,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallInfo {
    pub executable_path: Option<PathBuf>,
    pub installation_folder: Option<PathBuf>,
    pub uninstall_command: Option<String>,
    pub dist_type: DistType,
    pub create_shortcut_file: bool,
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
                false,
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
