//!Manages the database for installed packages

use crate::includes::{error::SengetErrors, package::Package};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

pub struct PackageDatabase {
    db_path: PathBuf,
}

impl PackageDatabase {
    pub fn new(root_dir: &Path) -> Result<PackageDatabase, SengetErrors> {
        let db_folder = root_dir.join("database");
        if !db_folder.is_dir() {
            fs::create_dir(&db_folder)?;
        }
        let db_path = db_folder.join("packages.json");
        let pd = PackageDatabase { db_path };
        if !pd.db_path.is_file() {
            File::create(&pd.db_path)?;
            pd.save_packages(Vec::new())?;
        }
        Ok(pd)
    }

    pub fn fetch_all_packages(&self) -> Result<Vec<Package>, SengetErrors> {
        let packages_str = fs::read_to_string(&self.db_path)?;
        let packages = serde_json::from_str(&packages_str)?;
        Ok(packages)
    }

    fn save_packages(&self, packages: Vec<Package>) -> Result<(), SengetErrors> {
        let updated_packages_str = serde_json::to_string_pretty(&packages)?;
        // Create instead of open with write permissions
        // incase some weirdo decides to delete the file as the program runs
        File::create(&self.db_path)?.write_all(updated_packages_str.as_bytes())?;
        Ok(())
    }
    pub fn find_package(&self, name: &str) -> Result<Option<Package>, SengetErrors> {
        let name_lower = name.to_lowercase();
        let packages = self.fetch_all_packages()?;
        let result = packages.into_iter().find(|p| {
            p.repo.name.to_lowercase() == name_lower
                || p.repo.full_name.to_lowercase() == name_lower
        });
        Ok(result)
    }
    fn find_package_index(&self, package: &Package, packages: &[Package]) -> Option<usize> {
        packages.iter().position(|p| p == package)
    }
    pub fn add_package(&self, package: Package) -> Result<(), SengetErrors> {
        let mut packages = self.fetch_all_packages()?;
        packages.push(package);
        self.save_packages(packages)
    }

    pub fn update_package(
        &self,
        old_package: &Package,
        updated_package: Package,
    ) -> Result<(), SengetErrors> {
        let mut packages = self.fetch_all_packages()?;
        let index = self.find_package_index(old_package, &packages).unwrap();
        packages[index] = updated_package;
        self.save_packages(packages)
    }

    pub fn remove_package(&self, package: &Package) -> Result<(), SengetErrors> {
        let mut packages = self.fetch_all_packages()?;
        let index = self.find_package_index(package, &packages).unwrap();
        packages.remove(index);
        self.save_packages(packages)
    }
}
