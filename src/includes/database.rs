//!Manages the database for installed packages

use crate::includes::package::Package;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};
use tinydb::{error::DatabaseError, Database};

pub struct PackageDBManager {
    db: Database<Package>,
}

impl PackageDBManager {
    pub fn get_db_file_path(root_dir: &Path) -> Result<PathBuf, io::Error> {
        let db_folder = root_dir.join("Package-Database");
        if !db_folder.is_dir() {
            fs::create_dir(&db_folder)?;
        }
        Ok(db_folder.join("packages.tinydb"))
    }
    pub fn new(save_path: &PathBuf) -> Result<PackageDBManager, DatabaseError> {
        let db = match save_path.is_file() {
            true => Database::<Package>::from(save_path),
            false => {
                let res = Database::<Package>::new("Senget", Some(save_path.clone()), true);
                Ok(res)
            }
        }?;
        Ok(PackageDBManager { db })
    }

    pub fn find_package<'a>(&'a self, name: &str) -> Result<Option<&'a Package>, DatabaseError> {
        let name_lower = name.to_lowercase();
        let package = self
            .db
            .query_item(|p| &p.lowercase_fullname, name_lower.clone());
        match package {
            Err(err) => match err {
                DatabaseError::ItemNotFound => {
                    match self.db.query_item(|p| &p.lowercase_name, name_lower) {
                        Err(err) => match err {
                            DatabaseError::ItemNotFound => Ok(None),
                            _ => Err(err),
                        },
                        Ok(p) => Ok(Some(p)),
                    }
                }
                _ => Err(err),
            },

            Ok(p) => Ok(Some(p)),
        }
    }

    pub fn remove_package(&mut self, package: &Package) -> Result<(), DatabaseError> {
        self.db.remove_item(package)?;
        self.db.dump_db()?;
        Ok(())
    }

    pub fn add_package(&mut self, package: Package) -> Result<(), DatabaseError> {
        self.db.add_item(package)?;
        self.db.dump_db()?;
        Ok(())
    }

    pub fn update_package(
        &mut self,
        old_package: &Package,
        new_package: Package,
    ) -> Result<(), DatabaseError> {
        self.db.update_item(old_package, new_package)?;
        self.db.dump_db()?;
        Ok(())
    }
    pub fn fetch_all_packages(&self) -> &HashSet<Package> {
        &self.db.items
    }
}

#[cfg(test)]
mod tests {
    use crate::includes::test_utils::{db_manager, senpwai_latest_package};

    #[test]
    fn test_adding_package() {
        let mut db_manager = db_manager();
        let added_package = senpwai_latest_package();
        db_manager
            .add_package(added_package.clone())
            .expect("Adding package");
        let found_package = db_manager
            .find_package(&added_package.lowercase_name)
            .unwrap()
            .unwrap();
        assert!(added_package == *found_package);
    }

    #[test]
    fn test_removing_package() {
        let mut db_manager = db_manager();
        let removed_package = senpwai_latest_package();
        db_manager.add_package(removed_package.clone()).unwrap();
        db_manager
            .remove_package(&removed_package)
            .expect("Removing package");
        assert!(db_manager
            .find_package(&removed_package.lowercase_name)
            .expect("Finding package")
            .is_none())
    }

    #[test]
    fn test_finding_package() {
        let mut db_manager = db_manager();
        let package_to_find = senpwai_latest_package();
        db_manager
            .add_package(package_to_find.clone())
            .expect("Adding package");
        let found_package = db_manager
            .find_package(&package_to_find.lowercase_name)
            .unwrap()
            .unwrap();
        assert_eq!(*found_package, package_to_find);
        let found_package = db_manager
            .find_package(&package_to_find.lowercase_fullname)
            .unwrap()
            .unwrap();
        assert_eq!(*found_package, package_to_find);
    }

    #[test]
    fn test_updating_package() {
        let mut db_manager = db_manager();
        let old_package = senpwai_latest_package();
        db_manager
            .add_package(old_package.clone())
            .expect("Adding package");
        let mut new_package = old_package.clone();
        new_package.version = "3.0.0".to_string();
        db_manager
            .update_package(&old_package, new_package.clone())
            .expect("Updating package");
        let found_package = db_manager
            .find_package(&new_package.repo.name)
            .unwrap()
            .unwrap();
        assert_eq!(*found_package, new_package);
    }
}

