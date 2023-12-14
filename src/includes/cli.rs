use crate::includes::commands::{
    download_installer, export_packages, exported_packages_filename, import_packages,
    install_package, list_packages, run_package, search_repos, show_package, uninstall_package,
};
use crate::includes::utils::{APP_NAME, DESCRIPTION, VERSION};
use crate::includes::error::KnownErrors;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

use super::commands::{clear_cached_installers, purge_packages, update_handler, Statics};

pub fn parse_commands() -> Command {
    let name_arg = Arg::new("name").help("Name of the package").required(true);
    let version_arg = Arg::new("version")
        .help("Version of the package")
        .default_value("latest");
    let folder_path_arg = |help: &str| {
        Arg::new("path")
            .default_value(".")
            .help(format!("Path to the folder{}", help))
    };
    let force_flag_arg = |help: &str| {
        Arg::new("force")
            .short('f')
            .long("force")
            .action(ArgAction::SetTrue)
            .help(help.to_owned())
    };

    let list_command = Command::new("list").about("List installed packages");
    let purge_command = Command::new("purge")
        .about("Remove packages that were uninstalled outside senget from the package database");
    let clear_cache_command = Command::new("clear-cache").about("Clear cached installers");
    let run_command = Command::new("run").about("Run a package").arg(&name_arg);
    let show_command = Command::new("show")
        .about("Show information about a package")
        .arg(&name_arg);
    let search_command = Command::new("search")
        .about("Search and list packages on github that match the specified name")
        .arg(&name_arg);
    let uninstall_command = Command::new("uninstall")
        .about("Uninstall a package")
        .arg(&name_arg)
        .arg(&force_flag_arg(
            "Remove the package from the package database even if automatic uninstallation fails",
        ));
    let install_command = Command::new("install")
        .about("Install a package")
        .arg(&name_arg)
        .arg(&version_arg);
    let download_command = Command::new("download")
        .about("Download the installer for a package")
        .arg(&name_arg)
        .arg(&version_arg)
        .arg(folder_path_arg(" to download the installer into"));
    let export_command = Command::new("export")
        .about(format!(
            "Export a list of installed packages to a file named {}",
            exported_packages_filename()
        ))
        .arg(folder_path_arg(" to save the export file into"));
    let import_command = Command::new("import")
        .about("Import a list of packages by installing")
        .arg(
            Arg::new("path")
                .help("Path to file containing the list of packages")
                // TODO: Update this in case I ever change crate::includes::commands::exported_packages_filename()
                .default_value("senget-packages.txt"),
        )
        .arg(
            Arg::new("ignore-versions")
                .long("ignore-versions")
                .short('i')
                .action(ArgAction::SetTrue)
                .help("Ignore the versions in the file and install the latest packages"),
        );
    let update_command = Command::new("update")
        .about("Update/Downgrade a package")
        .arg(&name_arg.default_value("all").required(false))
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
        .subcommand(clear_cache_command)
        .subcommand(purge_command)
}

pub async fn match_commands(
    commands: Command,
    statics: &mut Statics,
) -> Result<(), KnownErrors> {
    let get_string_value =
        |id: &str, arg_match: &ArgMatches| arg_match.get_one::<String>(id).unwrap().to_owned();
    let get_flag =
        |id: &str, arg_match: &ArgMatches| arg_match.get_one::<bool>(id).unwrap().to_owned();
    let get_name = |arg_match: &ArgMatches| get_string_value("name", arg_match);
    let get_version = |arg_match: &ArgMatches| get_string_value("version", arg_match);
    let get_path = |arg_match: &ArgMatches| PathBuf::from(get_string_value("path", arg_match));
    match commands.get_matches().subcommand() {
        Some(("list", _)) => Ok(list_packages(&statics.db)),
        Some(("purge", _)) => purge_packages(&mut statics.db),
        Some(("clear-cache", _)) => clear_cached_installers(&statics.installer_download_path),
        Some(("run", arg_match)) => run_package(&get_name(arg_match), &statics.db),
        Some(("show", arg_match)) => show_package(&get_name(arg_match),&statics.db, &statics.client).await,
        Some(("search", arg_match)) => search_repos(&get_name(arg_match), &statics.client).await,
        Some(("export", arg_match)) => export_packages(&get_path(arg_match), &statics.db),
        Some(("uninstall", arg_match)) => uninstall_package(
            &get_name(arg_match),
            get_flag("force", arg_match),
            &mut statics.db,
        ),
        Some(("download", arg_match)) => {
            download_installer(
                &get_name(arg_match),
                &get_version(arg_match),
                &get_path(arg_match),
                &statics.client,
                &statics.version_regex,
            )
            .await
        }

        Some(("install", arg_match)) => {
            install_package(
                &get_name(arg_match),
                &get_version(arg_match),
                statics
            ).await
        }
        Some(("update", arg_match)) => {
            update_handler(
                &get_name(&arg_match),
                &get_version(&arg_match),
                statics
            )
            .await
        }
        Some(("import", arg_match)) => {
            import_packages(
                &get_path(arg_match),
                get_flag("ignore-versions", arg_match),
                statics
            )
            .await
        }

        _ => Ok(eprintln!(
            "Invalid command. Use --help for usage information."
        )),
    }
}
