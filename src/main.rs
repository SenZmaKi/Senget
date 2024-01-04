//!Main program

mod includes;

use includes::{
    cli::{self, match_commands},
    commands::{validate_cache_folder_size, Statics},
    database::PackageDBManager,
    error::{print_error, KnownErrors},
    github, dist,
    senget_manager::{
        check_if_senget_update_available, generate_senget_package, setup_senget_package,
    },
    utils::{root_dir, DESCRIPTION, VERSION},
};

async fn run() -> Result<(), KnownErrors> {
    let commands = cli::parse_commands();
    let root_dir = root_dir();
    let statics = Statics::new(&root_dir)?;
    let db_save_path = PackageDBManager::get_db_file_path(&root_dir)?;
    let mut db = PackageDBManager::new(&db_save_path)?;
    let senget_package =
        generate_senget_package(root_dir.clone(), VERSION.to_owned(), DESCRIPTION.to_owned())?;
    setup_senget_package(&mut db, &senget_package, VERSION)?;
    let update_available =
        check_if_senget_update_available(&senget_package, &statics.client, &statics.version_regex);
    match_commands(commands, &mut db, &statics).await?;
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
        // Average error stack trace enjoyer: 🗿
        print_error(err)
    };
}
