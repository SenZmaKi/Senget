//!Contains shared test utility

#[cfg(test)]
pub use tests::*;

#[cfg(test)]
pub mod tests {
    use crate::includes::github::api::Repo;
    use crate::includes::dist::{InstallerDist, PackageInfo, DistType};
    use crate::includes::{database::PackageDatabase, utils};
    use crate::includes::{dist::InstallInfo, package::Package};
    use std::{fs, path::PathBuf};

    pub fn package_dist_dir() -> PathBuf {
        let p = PathBuf::from("test-distributables");
        if !p.is_dir() {
            fs::create_dir(&p).unwrap();
        }
        p
    }
    pub fn senpwai_repo() -> Repo {
        Repo::new(
            "Senpwai".to_owned(),
            "SenZmaKi/Senpwai".to_owned(),
            "https://github.com/SenZmaKi/Senpwai".to_owned(),
            Some("A desktop app for batch downloading anime".to_owned()),
            Some("Python".to_owned()),
            Some("GNU General Public License v3.0".to_owned()),
        )
    }

    pub fn hatt_repo() -> Repo {
        Repo::new(
            "Hatt".to_owned(),
            "Frenchgithubuser/Hatt".to_owned(),
            "https://github.com/frenchgithubuser/hatt".to_owned(),
            Some("DDL Meta search engine".to_owned()),
            Some("Go".to_owned()),
            Some("GNU General Public License v3.0".to_owned()),
        )
    }
    pub fn senpwai_latest_package() -> Package {
        senpwai_package("2.0.9".to_owned())
    }
    pub fn senpwai_package(version: String) -> Package {
        let install_info = InstallInfo {
            executable_path: Some(PathBuf::from(
                "C:\\Users\\PC\\AppData\\Local\\Programs\\Senpwai\\Senpwai.exe",
            )),
            installation_folder: Some(PathBuf::from(
                "C:\\Users\\PC\\AppData\\Local\\Programs\\Senpwai",
            )),
            uninstall_command: Some(
                "C:\\Users\\PC\\AppData\\Local\\Programs\\Senpwai\\unins000.exe /SILENT".to_owned(),
            ),
            dist_type: DistType::Installer,
            create_shortcut_file: false,
        };
        Package::new(version, senpwai_repo(), install_info)
    }
    pub fn hatt_package() -> Package {
        let install_info = InstallInfo {
            executable_path: Some(PathBuf::from("C:\\Users\\PC\\OneDrive\\Documents\\Rust\\Senget\\Packages\\Hatt\\hatt.exe")),
            installation_folder: Some(PathBuf::from("C:\\Users\\PC\\OneDrive\\Documents\\Rust\\Senget\\Packages\\Hatt")),
            uninstall_command: None,
            dist_type: DistType::Exe,
            create_shortcut_file: false,
        };
        Package::new("0.3.1".to_owned(), hatt_repo(), install_info)
    }

    fn setup_test_db_save_folder() -> PathBuf {
        let db_folder = PathBuf::from("test-database");
        if !db_folder.is_dir() {
            fs::create_dir(&db_folder).unwrap();
        }
        // Delete previous DB file cause each test assumes it's a clean start
        let f = db_folder.join("packages.json");
        if f.is_file() {
            fs::remove_file(&f).unwrap();
        }
        db_folder
    }

    pub fn senpwai_latest_dist() -> InstallerDist {
        let package_info = PackageInfo::new(
            "Senpwai".to_owned(),
            "https://github.com/SenZmaKi/Senpwai/releases/download/v2.0.9/Senpwai-setup.exe"
                .to_owned(),
            "2.0.9".to_owned(),
            "Senpwai-setup.exe".to_owned(),
        );
        InstallerDist { package_info }
    }

    pub fn db_manager() -> PackageDatabase {
        PackageDatabase::new(&setup_test_db_save_folder()).unwrap()
    }

    pub fn client() -> reqwest::Client {
        utils::setup_client().unwrap()
    }
}
