mod includes;

use includes::{
    cli::{self, match_commands},
    error::print_error,
    utils, commands::Statics,
    github,
    install

};
use tokio::runtime::Runtime;


fn main() {
    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(err) => {
            print_error(err.into());
            return;
        }
    };

    let commands = cli::parse_commands();

    rt.block_on(async {
        if let Err(err) = {
            let mut statics = match Statics::new() {
                Ok(ok) => ok,
                Err(err) => {
                    print_error(err);
                    return
                }
            };
            match_commands(
                commands,
                &mut statics
            ).await
        } {
            print_error(err);
        }
    });
}
