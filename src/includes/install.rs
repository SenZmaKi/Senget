//! Manages package download and installation

use crate::utils::{LoadingAnimation, RequestOrIOError};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use lnk::ShellLink;
use serde::{Serialize, Deserialize};
use std::{collections::HashSet, env, fs, io::Error as IOError, path::PathBuf, process::Command};
use tokio::io::AsyncWriteExt;
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey,
};

const SILENT_INSTALL_ARGS: [&str; 3] = [
    "/VERYSILENT", // Inno Setup
    "/qn",         // MSI
    "/S",          // NSIS
];

const UNINSTALL_KEY_STR: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
lazy_static! {
    static ref START_MENU_FOLDER: PathBuf = find_startmenu();
    static ref UNINSTALL_REG_KEY_MACHINE: RegKey = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(UNINSTALL_KEY_STR)
        .unwrap();
    static ref UNINSTALL_REG_KEY_USER: RegKey = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(UNINSTALL_KEY_STR)
        .unwrap();
}

fn find_startmenu() -> PathBuf {
    let appdata_path = env::var("APPDATA").unwrap();
    let path = appdata_path + "\\Microsoft\\Windows\\Start Menu\\Programs";
    PathBuf::from(path)
}

#[derive(Debug, Default, Clone)]
pub struct Installer {
    package_name: String,
    file_title: String,
    file_extension: String,
    pub url: String,
    pub version: String,
}
impl Installer {
    pub fn new(
        package_name: String,
        file_extension: String,
        url: String,
        version: String,
    ) -> Installer {
        let file_title = format!("{}-Installer.{}", package_name, file_extension);
        Installer {
            package_name,
            file_title,
            file_extension,
            url,
            version,
        }
    }
    pub async fn download(
        &self,
        path: &PathBuf,
        client: &reqwest::Client,
    ) -> Result<PathBuf, RequestOrIOError> {
        let path = path.join(&self.file_title);
        let mut file = tokio::fs::File::create(&path).await?;
        let mut response = client.get(&self.url).send().await?;
        let progress_bar = ProgressBar::new(response.content_length().unwrap());
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {bytes}/{total_bytes} ({eta} left)")
                .unwrap(),
        );
        let mut progress = 0;
        progress_bar.set_position(progress);
        progress_bar.set_message(format!(
            "Downloading {} v{}",
            self.package_name, self.version
        ));
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            progress += chunk.len() as u64;
            progress_bar.set_position(progress);
        }
        progress_bar.finish_with_message("Download complete");
        Ok(path)
    }
    fn fetch_shortcut_files(
        files: &mut HashSet<PathBuf>,
        check_inner_folders: bool,
    ) -> Result<(), IOError> {
        let entries = START_MENU_FOLDER.read_dir()?;
        for e in entries {
            match e {
                Ok(e) => {
                    let e = e.path();
                    if e.is_file() && e.ends_with(".lnk") {
                        files.insert(e);
                    } else if check_inner_folders && e.is_dir() {
                        Installer::fetch_shortcut_files(files, false)?;
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    fn fetch_reg_keys(parent_regkey: &RegKey) -> Result<HashSet<String>, IOError> {
        let mut subkeys = HashSet::<String>::new();
        for entry in parent_regkey.enum_keys() {
            match entry {
                Ok(subkey) => {
                    subkeys.insert(subkey.to_owned());
                }
                Err(err) => return Err(err),
            }
        }
        Ok(subkeys)
    }

    fn run_installation(file_path: &PathBuf) -> Result<(), std::io::Error> {
        Command::new(file_path).args(SILENT_INSTALL_ARGS).output()?;
        Ok(())
    }

    fn statically_generate_package_shortcut(&self) -> Option<PathBuf> {
        let shortcut_path = START_MENU_FOLDER.join(format!("{}.lnk", self.package_name));
        if shortcut_path.is_file() {
            Some(shortcut_path)
        } else {
            None
        }
    }

    fn find_shorcut_target(shortcut_path: &PathBuf) -> Option<PathBuf> {
        let lnk = ShellLink::open(shortcut_path).ok()?;
        let target = lnk.link_info().as_ref()?.local_base_path().as_ref()?;
        Some(PathBuf::from(target))
    }

    fn dynamically_find_package_shortcut(
        shortcut_files_before: &HashSet<PathBuf>,
    ) -> Option<PathBuf> {
        let mut shortcut_files_after = HashSet::<PathBuf>::new();
        Installer::fetch_shortcut_files(&mut shortcut_files_after, true).ok()?;

        let new_files = shortcut_files_after
            .difference(shortcut_files_before)
            .collect::<Vec<&PathBuf>>();

        new_files
            .first()
            .and_then(|f| Some(f.to_owned().to_owned()))
    }

    fn extract_uninstall_command_from_keys(
        new_keys: Vec<&String>,
        parent_regkey: &RegKey,
    ) -> Option<String> {
        if new_keys.is_empty() {
            return None;
        }

        let k = parent_regkey.open_subkey(new_keys[0]).ok()?;
        let command: String = k
            .get_value("QuietUninstallString")
            .or_else(|_| k.get_value("UninstallString"))
            .ok()?;
        Some(command)
    }
    fn fetch_uninstall_command_for_key(
        after_keys: &HashSet<String>,
        before_keys: &HashSet<String>,
        parent_regkey: &RegKey,
    ) -> Option<String> {
        let new_keys = after_keys.difference(before_keys).collect::<Vec<&String>>();
        Installer::extract_uninstall_command_from_keys(new_keys, parent_regkey)
    }

    fn fetch_uninstall_command(
        user_reg_keys_before: &HashSet<String>,
        machine_reg_keys_before: &HashSet<String>,
    ) -> Result<Option<String>, IOError> {
        let user_reg_keys_after = Installer::fetch_reg_keys(&UNINSTALL_REG_KEY_USER)?;
        let mut uninstall_command = Installer::fetch_uninstall_command_for_key(
            &user_reg_keys_after,
            &user_reg_keys_before,
            &UNINSTALL_REG_KEY_USER,
        );
        if uninstall_command.is_none() {
            let machine_reg_keys_after = Installer::fetch_reg_keys(&UNINSTALL_REG_KEY_MACHINE)?;
            uninstall_command = Installer::fetch_uninstall_command_for_key(
                &machine_reg_keys_after,
                &machine_reg_keys_before,
                &UNINSTALL_REG_KEY_MACHINE,
            );
        }
        Ok(uninstall_command)
    }

    pub fn install(
        &self,
        file_path: &PathBuf,
        loading_animation: &LoadingAnimation,
    ) -> Result<InstallInfo, IOError> {
        let join_handle = loading_animation.start(format!(
            "Installing {} v{}.. .",
            self.package_name, self.version
        ));
        let user_reg_keys_before = Installer::fetch_reg_keys(&UNINSTALL_REG_KEY_USER)?;
        let machine_reg_keys_before = Installer::fetch_reg_keys(&UNINSTALL_REG_KEY_MACHINE)?;
        let mut shortcut_files_before = HashSet::<PathBuf>::new();
        Installer::fetch_shortcut_files(&mut shortcut_files_before, true)?;

        Installer::run_installation(file_path)?;
        fs::remove_file(file_path)?;

        let executable_path = self
            .statically_generate_package_shortcut()
            .or_else(|| Installer::dynamically_find_package_shortcut(&shortcut_files_before))
            .and_then(|path| Installer::find_shorcut_target(&path));

        let uninstall_command =
            Installer::fetch_uninstall_command(&user_reg_keys_before, &machine_reg_keys_before)?;

        loading_animation.stop(join_handle);
        Ok(InstallInfo {
            executable_path,
            uninstall_command,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallInfo {
    pub executable_path: Option<PathBuf>,
    pub uninstall_command: Option<String>,
}
mod tests {
    use super::{Installer};
    use crate::{
        includes::utils::LOADING_ANIMATION,
        utils::{setup_client, PACKAGE_INSTALLER_DIR},
    };

    fn senpwai_installer() -> Installer {
        Installer::new(
            "Senpwai".to_owned(),
            "exe".to_owned(),
            "https://github.com/SenZmaKi/Senpwai/releases/download/v2.0.7/Senpwai-setup.exe"
                .to_owned(),
            "2.0.7".to_owned(),
        )
    }
    #[tokio::test]
    async fn test_downloading_installer() {
        let f_path = senpwai_installer()
            .download(&PACKAGE_INSTALLER_DIR, &setup_client())
            .await
            .expect("Successful Download");
        assert!(f_path.is_file());
    }

    #[test]
    fn test_installation() {
        let path = PACKAGE_INSTALLER_DIR.join("Senpwai-Installer.exe");
        let install_locations = senpwai_installer()
            .install(&path, &LOADING_ANIMATION)
            .expect("Successful Installation");
        println!("Results for test installation\n {:?}", install_locations);

        assert!(install_locations
            .executable_path
            .expect("Executable path to be Some")
            .is_file());
        assert!(install_locations.uninstall_command.is_some());
    }
}
