use crate::includes::commands::{
    download_installer, export_packages, exported_packages_file_name, import_packages,
    install_package, list_packages, run_package, search_repos, show_package, uninstall_package,
    update_package,
};
use crate::includes::utils::{APP_NAME, DESCRIPTION, VERSION};
use crate::includes::{database::PackageDBManager, error::KnownErrors, utils::LoadingAnimation};
use clap::{Arg, ArgAction, ArgMatches, Command};
use regex::Regex;
use reqwest::Client;
use std::path::PathBuf;
use winreg::RegKey;

pub fn parse_commands() -> Command {
    let name_arg = Arg::new("name").help("Name of the package").required(true);
    let version_arg = Arg::new("version")
        .help("Version of the package")
        .default_value("latest");
    let folder_path_arg = |help: &str| Arg::new("path").help(format!("Path to the folder{}", help));

    let list_command = Command::new("list").about("List installed packages");
    let run_command = Command::new("run").about("Run a package").arg(&name_arg);
    let show_command = Command::new("show")
        .about("Show information about a package")
        .arg(&name_arg);
    let search_command = Command::new("search")
        .about("Search and list packages on github that match the specified name")
        .arg(&name_arg);
    let uninstall_command = Command::new("uninstall")
        .about("Uninstall a package")
        .arg(&name_arg);
    let install_command = Command::new("install")
        .about("Install a package")
        .arg(&name_arg)
        .arg(&version_arg);
    let download_command = Command::new("download")
        .about("Download the installer for a package")
        .arg(&name_arg)
        .arg(folder_path_arg(" to download the installer into").required(true))
        .arg(&version_arg);
    let export_command = Command::new("export")
        .about(format!(
            "Export a list of installed packages to a file named {}",
            exported_packages_file_name()
        ))
        .arg(folder_path_arg(" to save the export file into").default_value("."));
    let import_command = Command::new("import")
        .about("Import a list of packages by installing")
        .arg(
            Arg::new("path")
                .help("Path to file containing the list of packages")
                .required(true),
        )
        .arg(
            Arg::new("ignore-versions")
                .long("ignore-versions")
                .short('i')
                .action(ArgAction::SetTrue)
                .help("Whether to ignore the versions in the file and install the latest packages"),
        );
    let update_command = Command::new("update")
        .about("Update/Downgrade a package")
        .arg(&name_arg)
        .arg(
            Arg::new("version")
                .help("Version to update/downgrade to")
                .default_value("latest"),
        );

    Command::new(APP_NAME)
        .version(VERSION)
        .about(DESCRIPTION)
        .subcommand(show_command)
        .subcommand(install_command)
        .subcommand(update_command)
        .subcommand(uninstall_command)
        .subcommand(download_command)
        .subcommand(list_command)
        .subcommand(search_command)
        .subcommand(run_command)
        .subcommand(export_command)
        .subcommand(import_command)
}

pub async fn match_commands(
    commands: Command,
    db: &mut PackageDBManager,
    client: &Client,
    installer_download_path: &PathBuf,
    version_regex: &Regex,
    loading_animation: &LoadingAnimation,
    startmenu_folder: &PathBuf,
    user_uninstall_reg_key: &RegKey,
    machine_uninstall_reg_key: &RegKey,
) -> Result<(), KnownErrors> {
    let get_string_value =
        |id: &str, arg_match: &ArgMatches| arg_match.get_one::<String>(id).unwrap().to_owned();
    let get_flag =
        |id: &str, arg_match: &ArgMatches| arg_match.get_one::<bool>(id).unwrap().to_owned();
    let get_name = |arg_match: &ArgMatches| get_string_value("name", arg_match);
    let get_version = |arg_match: &ArgMatches| get_string_value("version", arg_match);
    let get_path = |arg_match: &ArgMatches| PathBuf::from(get_string_value("path", arg_match));
    match commands.get_matches().subcommand() {
        Some(("list", _)) => Ok(list_packages(db)),
        Some(("run", arg_match)) => run_package(db, &get_name(arg_match)),
        Some(("show", arg_match)) => show_package(db, &get_name(arg_match), client).await,
        Some(("search", arg_match)) => search_repos(&get_name(arg_match), client).await,
        Some(("export", arg_match)) => export_packages(db, &get_path(arg_match)),
        Some(("uninstall", arg_match)) => {
            uninstall_package(db, &get_name(arg_match), loading_animation)
        }
        Some(("download", arg_match)) => {
            download_installer(
                &get_name(arg_match),
                client,
                &get_version(arg_match),
                version_regex,
                &get_path(arg_match),
            )
            .await
        }

        Some(("install", arg_match)) => {
            install_package(
                db,
                &get_name(arg_match),
                client,
                &get_version(arg_match),
                version_regex,
                installer_download_path,
                loading_animation,
                startmenu_folder,
                user_uninstall_reg_key,
                machine_uninstall_reg_key,
            )
            .await
        }
        Some(("update", arg_match)) => {
            update_package(
                db,
                &get_name(&arg_match),
                client,
                &get_version(&arg_match),
                version_regex,
                installer_download_path,
                loading_animation,
                startmenu_folder,
                user_uninstall_reg_key,
                machine_uninstall_reg_key,
            )
            .await
        }
        Some(("import", arg_match)) => {
            import_packages(
                &get_path(arg_match),
                get_flag("ignore-versions", arg_match),
                db,
                client,
                version_regex,
                installer_download_path,
                loading_animation,
                startmenu_folder,
                user_uninstall_reg_key,
                machine_uninstall_reg_key,
            )
            .await
        }

        _ => Ok(eprintln!(
            "Invalid command. Use --help for usage information."
        )),
    }
}
