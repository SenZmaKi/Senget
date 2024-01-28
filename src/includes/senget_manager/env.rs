//!Manages the SENGET_PACKAGES environment variable

use std::io;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

const SENGET_PACKAGES_ENV_VAR: &str = "SENGET_PACKAGES";

pub fn setup_senget_packages_path_env_var(
    senget_installation_folder: &str,
) -> Result<(), io::Error> {
    let env_var = open_env_var()?;
    let path_value = env_var.get_value::<String, _>("Path")?;
    if !path_value.contains(SENGET_PACKAGES_ENV_VAR) {
        let updated_value = format!(
            "{};%{}%",
            env_var.get_value::<String, _>("Path")?,
            SENGET_PACKAGES_ENV_VAR
        );
        env_var.set_value("Path", &updated_value)?;
    }
    if let Err(err) = env_var.get_value::<String, _>(SENGET_PACKAGES_ENV_VAR) {
        if err.kind() == io::ErrorKind::NotFound {
            set_senget_env_var_value(&env_var, senget_installation_folder)?;
        }
        return Err(err);
    }
    Ok(())
}
pub fn add_package_folder_to_senget_env_var(
    package_installation_folder: &str,
) -> Result<(), io::Error> {
    add_senget_env_var_value(&open_env_var()?, package_installation_folder)
}

pub fn remove_package_folder_from_senget_env_var(
    package_installation_folder: &str,
) -> Result<(), io::Error> {
    let env_var = open_env_var()?;
    let initial_value = get_senget_env_var_value(&env_var)?;
    let to_replace = format!(";{}", package_installation_folder);
    let new_value = initial_value.replace(&to_replace, "");
    set_senget_env_var_value(&env_var, &new_value)
}
fn get_senget_env_var_value(env_var: &RegKey) -> Result<String, io::Error> {
    env_var.get_value(SENGET_PACKAGES_ENV_VAR)
}

fn set_senget_env_var_value(env_var: &RegKey, value: &str) -> Result<(), io::Error> {
    env_var.set_value(SENGET_PACKAGES_ENV_VAR, &value)
}

fn add_senget_env_var_value(env_var: &RegKey, new_value: &str) -> Result<(), io::Error> {
    let updated_value = format!("{};{}", get_senget_env_var_value(env_var)?, new_value);
    set_senget_env_var_value(env_var, &updated_value)
}

fn open_env_var() -> Result<RegKey, io::Error> {
    // create_subkey instead of open with KEY_ALL_ACCESS incase some weirdo doesn't have Environment
    // path variable, will probably never happen but my anxiety
    let (env, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey("Environment")?;
    Ok(env)
}
