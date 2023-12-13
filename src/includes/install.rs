//! Manages package download and installation

use indicatif::{ProgressBar, ProgressStyle};
use lnk::ShellLink;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    env, fs,
    io::{self, Error as IOError},
    path::PathBuf,
    process::Command,
};
use tokio::io::AsyncWriteExt;
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey,
};

use crate::includes::{
    error::{ContentLengthError, RequestIoContentLengthError},
    utils::{display_path, LoadingAnimation, DEBUG, MSI_EXEC},
};

// Running an msi installer that needs admin access silently is problematic since it won't tell us that it failed
// const MSI_SILENT_ARG: &str = "/qn";
const INNO_SILENT_ARG: &str = "/VERYSILENT";
const NSIS_SILENT_ARG: &str = "/S";
const STARTMENU_FOLDER_ENDPOINT: &str = "\\Microsoft\\Windows\\Start Menu\\Programs";

#[derive(Debug, Default, Clone)]
pub struct Installer {
    package_name: String,
    file_title: String,
    pub file_extension: String,
    pub url: String,
    pub version: String,
}
impl Installer {
    const UNINSTALL_KEY_STR: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    pub fn generate_installer_download_path() -> Result<PathBuf, io::Error> {
        let install_download_path = PathBuf::from("Package-Installers");
        if !install_download_path.is_dir() {
            fs::create_dir(&install_download_path)?;
        }
        Ok(install_download_path)
    }
    pub fn new(
        package_name: String,
        file_extension: String,
        url: String,
        version: String,
    ) -> Installer {
        let file_title = format!("{}-v{}-Installer.{}", package_name, version, file_extension);
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
    ) -> Result<PathBuf, RequestIoContentLengthError> {
        let path = path.join(&self.file_title);
        if path.is_file() {
            println!(
                "Using cached installer at: {}",
                display_path(&path).unwrap_or_default()
            );
            return Ok(path);
        }
        let mut file = tokio::fs::File::create(&path).await?;
        let mut response = client.get(&self.url).send().await?;
        let progress_bar = ProgressBar::new(
            response
                .content_length()
                .ok_or_else(|| ContentLengthError)?,
        );
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {bytes}/{total_bytes} ({eta} left)")
                .expect("Valid template"),
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
        // So that further output is after the progressbar
        println!();
        Ok(path)
    }

    pub fn generate_machine_uninstall_reg_key() -> Result<RegKey, io::Error> {
        Ok(RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(Installer::UNINSTALL_KEY_STR)?)
    }

    pub fn generate_user_uninstall_reg_key() -> Result<RegKey, io::Error> {
        Ok(RegKey::predef(HKEY_CURRENT_USER).open_subkey(Installer::UNINSTALL_KEY_STR)?)
    }

    pub fn generate_startmenu_paths() -> (PathBuf, PathBuf) {
        let gen = |envvar: &str| {
            PathBuf::from(
                env::var(envvar).expect(&format!("{} environment variable to be set", envvar))
                    + STARTMENU_FOLDER_ENDPOINT,
            )
        };
        (gen("APPDATA"), gen("PROGRAMDATA"))
    }
    fn fetch_shortcut_files(
        files: &mut HashSet<PathBuf>,
        startmenu_folder: &PathBuf,
        check_inner_folders: bool,
    ) -> Result<(), IOError> {
        for e in startmenu_folder.read_dir()? {
            match e {
                Ok(e) => {
                    let e = e.path();
                    if e.is_file() && e.ends_with(".lnk") {
                        files.insert(e);
                    } else if check_inner_folders && e.is_dir() {
                        Installer::fetch_shortcut_files(files, startmenu_folder, false)?;
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

    fn run_installation(&self, file_path: &PathBuf) -> Result<(), std::io::Error> {
        match self.file_extension == "msi" {
            true => Command::new(MSI_EXEC).arg("/i").arg(file_path).output()?,
            false => Command::new(file_path)
                .args([INNO_SILENT_ARG, NSIS_SILENT_ARG])
                .output()?,
        };
        Ok(())
    }

    fn statically_generate_package_shortcut(&self, startmenu_folder: &PathBuf) -> Option<PathBuf> {
        let shortcut_file_name = format!("{}.lnk", &self.package_name);
        let shortcut_path = startmenu_folder.join(&shortcut_file_name);
        if shortcut_path.is_file() {
            return Some(shortcut_path);
        }
        let shortcut_path = startmenu_folder
            .join(&self.package_name)
            .join(shortcut_file_name);
        if shortcut_path.is_file() {
            return Some(shortcut_path);
        }
        None
    }

    fn find_shorcut_target(shortcut_path: &PathBuf) -> Option<PathBuf> {
        let lnk = ShellLink::open(shortcut_path).ok()?;
        let target = lnk.link_info().as_ref()?.local_base_path().as_ref()?;
        Some(PathBuf::from(target))
    }

    fn dynamically_find_package_shortcut(
        shortcut_files_before: &HashSet<PathBuf>,
        startmenu_folder: &PathBuf,
    ) -> Option<PathBuf> {
        let mut shortcut_files_after = HashSet::<PathBuf>::new();
        Installer::fetch_shortcut_files(&mut shortcut_files_after, startmenu_folder, true).ok()?;

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
        installation_folder: &Option<PathBuf>,
        user_reg_keys_before: &HashSet<String>,
        machine_reg_keys_before: &HashSet<String>,
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<Option<String>, IOError> {
        let user_reg_keys_after = Installer::fetch_reg_keys(user_uninstall_reg_key)?;
        let mut uninstall_command = Installer::fetch_uninstall_command_for_key(
            &user_reg_keys_after,
            &user_reg_keys_before,
            &user_uninstall_reg_key,
        );
        if uninstall_command.is_none() {
            let machine_reg_keys_after = Installer::fetch_reg_keys(machine_uninstall_reg_key)?;
            uninstall_command = Installer::fetch_uninstall_command_for_key(
                &machine_reg_keys_after,
                &machine_reg_keys_before,
                &machine_uninstall_reg_key,
            );
        }
        if uninstall_command.is_none() && installation_folder.is_some() {
            for e in installation_folder
                .as_ref()
                .expect("installation_folder.is_some()")
                .read_dir()?
            {
                if let Ok(e) = e {
                    let file_name = e.file_name().to_str().unwrap().to_lowercase();
                    if file_name.contains("unins") && file_name.ends_with(".exe") {
                        uninstall_command = Some(display_path(&e.path())?);
                    }
                }
            }
        }
        Ok(uninstall_command)
    }

    pub fn install(
        &self,
        installer_path: &PathBuf,
        loading_animation: &mut LoadingAnimation,
        startmenu_folders: &(PathBuf, PathBuf),
        user_uninstall_reg_key: &RegKey,
        machine_uninstall_reg_key: &RegKey,
    ) -> Result<InstallInfo, IOError> {
        loading_animation.start(format!("Installing {}.. .", self.package_name));
        let user_reg_keys_before = Installer::fetch_reg_keys(user_uninstall_reg_key)?;
        let machine_reg_keys_before = Installer::fetch_reg_keys(machine_uninstall_reg_key)?;
        let mut shortcut_files_before = HashSet::<PathBuf>::new();
        Installer::fetch_shortcut_files(&mut shortcut_files_before, &startmenu_folders.0, true)?;
        Installer::fetch_shortcut_files(&mut shortcut_files_before, &startmenu_folders.1, true)?;

        self.run_installation(installer_path)?;
        if !DEBUG {
            fs::remove_file(installer_path)?;
        }

        let executable_path = self
            .statically_generate_package_shortcut(&startmenu_folders.0)
            .or_else(|| self.statically_generate_package_shortcut(&startmenu_folders.1))
            .or_else(|| {
                Installer::dynamically_find_package_shortcut(
                    &shortcut_files_before,
                    &startmenu_folders.0,
                )
                .or_else(|| {   
                    Installer::dynamically_find_package_shortcut(
                        &shortcut_files_before,
                        &startmenu_folders.1,
                    )
                })
            })
            .and_then(|path| Installer::find_shorcut_target(&path));

        let installation_folder = executable_path
            .as_ref()
            .and_then(|ep| ep.parent().map(|p| PathBuf::from(p)));

        let uninstall_command = Installer::fetch_uninstall_command(
            &installation_folder,
            &user_reg_keys_before,
            &machine_reg_keys_before,
            user_uninstall_reg_key,
            machine_uninstall_reg_key,
        )?;

        loading_animation.stop();
        Ok(InstallInfo {
            executable_path,
            installation_folder,
            uninstall_command,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallInfo {
    pub executable_path: Option<PathBuf>,
    pub installation_folder: Option<PathBuf>,
    pub uninstall_command: Option<String>,
}

mod tests {
    use crate::includes::{
        install::Installer,
        test_utils::{client, loading_animation, package_installers_dir, senpwai_latest_installer},
    };

    #[tokio::test]
    async fn test_downloading_installer() {
        let f_path = senpwai_latest_installer()
            .download(&package_installers_dir(), &client())
            .await
            .expect("Downloading");
        assert!(f_path.is_file());
    }

    #[test]
    fn test_installation() {
        let path = package_installers_dir().join("Senpwai-Installer.exe");
        let install_info = senpwai_latest_installer()
            .install(
                &path,
                &mut loading_animation(),
                &&Installer::generate_startmenu_paths(),
                &Installer::generate_user_uninstall_reg_key().expect("Ok(user_uninstall_reg_key)"),
                &Installer::generate_machine_uninstall_reg_key()
                    .expect("Ok(machine_uninstall_reg_key)"),
            )
            .expect("Ok(install_info)");
        println!("Results for test_installation\n {:?}", install_info);

        assert!(install_info
            .executable_path
            .expect("Some(executable_path)")
            .is_file());
        assert!(install_info.uninstall_command.is_some());
    }
}
