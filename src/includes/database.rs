//! Manages the database for installed packages
use crate::includes::package::Package;
use crate::utils::APP_NAME;
use std::{fs, io, path::PathBuf};
use tinydb::{error::DatabaseError, Database};

pub struct PackageDBManager {
    db: Database<Package>,
}

impl PackageDBManager {
    pub fn get_db_file_path() -> Result<PathBuf, io::Error> {
        let db_folder = PathBuf::from("Database");
        if !db_folder.is_dir() {
            fs::create_dir(&db_folder)?;
        }
        Ok(db_folder.join("Packages.tinydb").canonicalize()?)
    }
    pub fn new(save_path: &PathBuf) -> Result<PackageDBManager, DatabaseError> {
        let db = match save_path.is_file() {
            true => Database::<Package>::from(save_path),
            false => {
                let res = Database::<Package>::new(APP_NAME, Some(save_path.to_owned()), true);
                Ok(res)
            }
        }?;
        Ok(PackageDBManager { db })
    }

    pub fn find_package<'a>(&'a self, name: &str) -> Result<Option<&'a Package>, DatabaseError> {
        let package = self
            .db
            .query_item(|p| &(&p.lowercase_name), name.to_lowercase());
        match package {
            Err(err) => match err {
                DatabaseError::ItemNotFound => Ok(None),
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
        println!("{:?}", self.db.save_path);
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
}

#[cfg(test)]
mod tests {
    use crate::includes::database::PackageDBManager;
    use crate::includes::package::Package;
    use crate::utils::SENPWAI_PACKAGE;
    use std::fs;
    use std::path::PathBuf;
    const FIND_PCKG_MSG: &str = "Finding package";
    fn setup_db_save_path() -> PathBuf {
        let db_folder = PathBuf::from("Test-Database");
        if !db_folder.is_dir() {
            fs::create_dir(&db_folder).expect("Making Test-Database folder");
        }
        // Delete previous DB file cause each test assumes it's a clean start
        let f = db_folder.join("test.tinydb");
        if f.is_file() {
            fs::remove_file(&f).expect("Removing previous DB file");
        }
        f
    }
    fn make_db_manager() -> PackageDBManager {
        PackageDBManager::new(&setup_db_save_path()).expect("Making PackageDBManager struct")
    }
    fn senpwai() -> Package {
        (*SENPWAI_PACKAGE).to_owned()
    }

    #[test]
    fn test_adding_package() {
        let mut db_manager = make_db_manager();
        let added_package = senpwai();
        db_manager
            .add_package(added_package.to_owned())
            .expect("Adding package");
        let found_package = db_manager
            .find_package(&added_package.lowercase_name)
            .expect(FIND_PCKG_MSG)
            .expect("Checking for valid package");
        assert!(added_package == *found_package);
    }

    #[test]
    fn test_removing_package() {
        let mut db_manager = make_db_manager();
        let removed_package = senpwai();
        db_manager
            .add_package(removed_package.to_owned())
            .expect("Adding package");
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
        let mut db_manager = make_db_manager();
        let package_to_find = senpwai();
        db_manager
            .add_package(package_to_find.to_owned())
            .expect("Adding package");
        let found_package = db_manager
            .find_package(&package_to_find.lowercase_name)
            .expect(FIND_PCKG_MSG)
            .expect("Checking for valid package");
        assert_eq!(*found_package, package_to_find);
    }

    #[test]
    fn test_updating_package() {
        let mut db_manager = make_db_manager();
        let old_package = senpwai();
        db_manager
            .add_package(old_package.to_owned())
            .expect("Adding package");
        let mut new_package = old_package.to_owned();
        new_package.version = "3.0.0".to_string();
        db_manager
            .update_package(&old_package, new_package.to_owned())
            .expect("Updating package");
        let found_package = db_manager
            .find_package(&new_package.repo.name)
            .expect(FIND_PCKG_MSG)
            .expect("Checking for valid package");
        assert_eq!(*found_package, new_package);
    }
}
