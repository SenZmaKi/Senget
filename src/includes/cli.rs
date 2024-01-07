//!Parses passed commands and arguments

use crate::includes::commands::{
    download_package, export_packages, import_packages, install_package, list_packages,
    run_package, search_repos, show_package, uninstall_package,
};
use crate::includes::error::KnownErrors;
use crate::includes::utils::{DESCRIPTION, VERSION};
use clap::builder::EnumValueParser;
use clap::{Arg, ArgAction, ArgMatches, Command};
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
    let path_arg = |help: &str| {
        Arg::new("path")
            .default_value(".")
            .help(format!("Path to {}", help))
    };
    let folder_path_arg = |help: &str| path_arg(&format!("the folder {}", help));
    let flag_arg = |help: &'static str, name: &'static str, short: char| {
        Arg::new(name)
            .short(short)
            .long(name)
            .help(help)
            .action(ArgAction::SetTrue)
    };
    let force_flag_arg = |help: &'static str| flag_arg(help, "force", 'f');

    let dist_type_arg = Arg::new("dist")
        .value_parser(EnumValueParser::<DistType>::new())
        .short('d')
        .long("dist")
        .help("Distribution type to install/download");
    let list_command = Command::new("list").about("List installed packages");
    let purge_command = Command::new("purge")
        .about("Remove packages that were uninstalled outside senget from the package database");
    let clear_cache_command = Command::new("clear-cache").about("Clear cached distributables");
    let run_command = Command::new("run")
        .about("Run a package")
        .arg(&name_arg)
        .arg(flag_arg(
            "Exit immediately after starting the package",
            "no-wait",
            'n',
        ))
        .arg(
            Arg::new("args")
                .short('a')
                .long("args")
                .num_args(0..)
                .allow_hyphen_values(true)
                .help("Arguments to pass to the package"),
        );
    let show_command = Command::new("show")
        .about("Show information about a package")
        .arg(&name_arg);
    let search_command = Command::new("search")
        .about("Search and list packages that match the specified name")
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
            path_arg("to the export file containing the list of packages")
                .default_value(EXPORTED_PACKAGES_FILENAME),
        )
        .arg(
            Arg::new("ignore-versions")
                .short('i')
                .long("ignore-versions")
                .action(ArgAction::SetTrue)
                .help("Install the latest versions instead of the versions in the file"),
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
fn get_name(arg_match: &ArgMatches) -> &str {
    get_string_value("name", arg_match)
}
fn get_flag(id: &str, arg_match: &ArgMatches) -> bool {
    *arg_match.get_one::<bool>(id).unwrap()
}
fn get_version(arg_match: &ArgMatches) -> &str {
    get_string_value("version", arg_match)
}
fn get_path(arg_match: &ArgMatches) -> PathBuf {
    PathBuf::from(get_string_value("path", arg_match))
}
fn get_dist_type(arg_match: &ArgMatches) -> Option<&DistType> {
    arg_match.get_one("dist")
}

fn get_string_vector<'a>(id: &str, arg_match: &'a ArgMatches) -> Vec<&'a String> {
    arg_match
        .get_many::<String>(id)
        .unwrap_or_default()
        .collect()
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
        Some(("run", arg_match)) => run_package(
            get_name(arg_match),
            get_flag("no-wait", arg_match),
            &get_string_vector("args", arg_match),
            db,
        ),
        Some(("show", arg_match)) => show_package(get_name(arg_match), db, &statics.client).await,
        Some(("search", arg_match)) => search_repos(get_name(arg_match), &statics.client).await,
        Some(("export", arg_match)) => export_packages(&get_path(arg_match), db),
        Some(("uninstall", arg_match)) => uninstall_package(
            get_name(arg_match),
            get_flag("force", arg_match),
            &statics.startmenu_folders.appdata,
            db,
        ),
        Some(("download", arg_match)) => {
            download_package(
                get_name(arg_match),
                get_version(arg_match),
                &statics.client,
                &statics.version_regex,
                &get_path(arg_match),
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
            update_handler(get_name(arg_match), get_version(arg_match), db, statics).await
        }
        Some(("import", arg_match)) => {
            import_packages(
                &get_path(arg_match),
                get_flag("ignore-versions", arg_match),
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

