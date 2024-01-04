//!Parses passed commands and arguments

use crate::includes::commands::{
    download_package, export_packages, import_packages, install_package, list_packages,
    run_package, search_repos, show_package, uninstall_package,
};
use crate::includes::error::KnownErrors;
use crate::includes::utils::{DESCRIPTION, VERSION};
use clap::builder::{EnumValueParser, PossibleValuesParser, ValueParser};
use clap::{Arg, ArgAction, ArgMatches, Command, ValueEnum};
use std::path::PathBuf;

use super::commands::{clear_cached_distributables, purge_packages, update_handler, Statics};
use super::database::PackageDBManager;
use super::dist::DistType;
use super::utils::EXPORTED_PACKAGES_FILENAME;

pub fn parse_commands() -> Command {
    let name_arg = Arg::new("name").help("Name of the package").required(true);
    let version_arg = Arg::new("version")
        .short('v')
        .long("version")
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
    let dist_type_arg = Arg::new("dist")
        .value_parser(EnumValueParser::<DistType>::new())
        .short('d')
        .long("dist")
        .help("Distribution type to install/download");
    let list_command = Command::new("list").about("List installed packages");
    let purge_command = Command::new("purge")
        .about("Remove packages that were uninstalled outside senget from the package database");
    let clear_cache_command = Command::new("clear-cache").about("Clear cached distributables");
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
        .arg(&version_arg)
        .arg(&dist_type_arg);
    let download_command = Command::new("download")
        .about("Download the distributable for a package")
        .arg(&name_arg)
        .arg(&version_arg)
        .arg(&dist_type_arg)
        .arg(folder_path_arg(" to download the distributable into"));
    let export_command = Command::new("export")
        .about(format!(
            "Export a list of installed packages to a file named {}",
            EXPORTED_PACKAGES_FILENAME
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
                .short('v')
                .long("version")
                .help("Version to update/downgrade to")
                .default_value("latest"),
        );

    Command::new("Senget")
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

fn get_string_value<'a>(id: &str, arg_match: &'a ArgMatches) -> &'a str {
    arg_match.get_one::<String>(id).unwrap()
}
fn get_name<'a>(arg_match: &'a ArgMatches) -> &'a str {
    get_string_value("name", arg_match)
}
fn get_flag(id: &str, arg_match: &ArgMatches) -> bool {
    *arg_match.get_one::<bool>(id).unwrap()
}
fn get_version<'a>(arg_match: &'a ArgMatches) -> &'a str {
    get_string_value("version", arg_match)
}
fn get_path(arg_match: &ArgMatches) -> PathBuf {
    PathBuf::from(get_string_value("path", &arg_match))
}
fn get_dist_type<'a>(arg_match: &'a ArgMatches) -> Option<&'a DistType> {
    arg_match.get_one("dist")
}
pub async fn match_commands(
    commands: Command,
    db: &mut PackageDBManager,
    statics: &Statics,
) -> Result<(), KnownErrors> {
    match commands.get_matches().subcommand() {
        Some(("list", _)) => {
            list_packages(db);
            Ok(())
        }
        Some(("purge", _)) => purge_packages(db),
        Some(("clear-cache", _)) => clear_cached_distributables(&statics.dists_folder_path),
        Some(("run", arg_match)) => run_package(get_name(arg_match), db),
        Some(("show", arg_match)) => show_package(get_name(arg_match), db, &statics.client).await,
        Some(("search", arg_match)) => search_repos(get_name(arg_match), &statics.client).await,
        Some(("export", arg_match)) => export_packages(&get_path(arg_match), db),
        Some(("uninstall", arg_match)) => uninstall_package(
            &get_name(arg_match),
            get_flag("force", arg_match).clone(),
            db,
        ),
        Some(("download", arg_match)) => {
            download_package(
                get_name(arg_match),
                get_version(arg_match),
                &statics.client,
                &statics.version_regex,
                &get_path(arg_match),
                &statics.dists_folder_path,
                &None,
            )
            .await
        }

        Some(("install", arg_match)) => {
            install_package(
                get_name(arg_match),
                get_version(arg_match),
                &get_dist_type(arg_match).cloned(),
                db,
                statics,
            )
            .await
        }
        Some(("update", arg_match)) => {
            update_handler(&get_name(arg_match), &get_version(arg_match), db, statics).await
        }
        Some(("import", arg_match)) => {
            import_packages(
                &get_path(arg_match),
                get_flag("ignore-versions", arg_match).clone(),
                db,
                statics,
            )
            .await
        }

        _ => Ok(eprintln!(
            "Invalid command. Use --help for usage information."
        )),
    }
}

