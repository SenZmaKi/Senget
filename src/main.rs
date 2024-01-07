//!Main program

mod includes;

use includes::{
    cli::{self, match_commands},
    commands::{validate_cache_folder_size, Statics},
    database::PackageDatabase,
    dist,
    error::{print_error, SengetErrors},
    github,
    senget_manager::{
        env::setup_senget_packages_path_env_var,
        package::{
            check_if_senget_update_available, generate_senget_package, setup_senget_package,
        },
    },
    utils::{root_dir, PathStr, DESCRIPTION, VERSION},
};

async fn run() -> Result<(), SengetErrors> {
    let commands = cli::parse_commands();
    let root_dir = root_dir();
    let statics = Statics::new(&root_dir)?;
    let db = PackageDatabase::new(&root_dir)?;
    let senget_package =
        generate_senget_package(root_dir.clone(), VERSION.to_owned(), DESCRIPTION.to_owned())?;
    setup_senget_package(&db, &senget_package, VERSION)?;
    setup_senget_packages_path_env_var(
        &senget_package
            .install_info
            .installation_folder
            .as_ref()
            .unwrap()
            .path_str()?,
    )?;
    let update_available =
        check_if_senget_update_available(&senget_package, &statics.client, &statics.version_regex);
    match_commands(commands, &db, &statics).await?;
    validate_cache_folder_size(&root_dir)?;
    if update_available.await? {
        println!("Senget update available, run \"senget update senget\" to update");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        // Absolute gigachad error handling
        // Average something went wrong fan: 🤓
        // Average full error stack trace enjoyer: 🗿
        print_error(err)
    };
}
