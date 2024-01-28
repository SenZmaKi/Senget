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
use std::sync::Arc;



fn init() -> Result<
    (
        clap::Command,
        Statics,
        PackageDatabase,
        includes::package::Package,
    ),
    SengetErrors,
> {
    let commands = cli::parse_commands();
    let root = root_dir();
    let statics = Statics::new(&root)?;
    let db = PackageDatabase::new(&root)?;
    let senget_package =
        generate_senget_package(root.clone(), VERSION.to_owned(), DESCRIPTION.to_owned())?;
    setup_senget_package(&db, &senget_package, VERSION)?;
    setup_senget_packages_path_env_var(
        &senget_package
            .install_info
            .installation_folder
            .as_ref()
            .unwrap()
            .path_str()?,
    )?;
    Ok((commands, statics, db, senget_package))
}

async fn run() -> Result<(), SengetErrors> {
    let (commands, statics, db, senget_package) = init()?;
    let statics_arc = Arc::new(statics);
    let statics_arc_ref_1 = Arc::clone(&statics_arc);
    let statics_arc_ref_2 = Arc::clone(&statics_arc);
    let (senget_result, update_available) = tokio::join!(
        tokio::spawn(async move { match_commands(commands, &db, &statics_arc_ref_1).await }),
        tokio::spawn(async move {
            check_if_senget_update_available(
                &senget_package,
                &statics_arc_ref_2.client,
                &statics_arc_ref_2.version_regex,
            )
            .await
        })
    );
    senget_result.unwrap()?;
    validate_cache_folder_size(&statics_arc.dists_folder_path)?;
    if update_available.unwrap()? {
        println!("Senget update available, run \"senget update senget\" to update");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        print_error(err)
    }
}
