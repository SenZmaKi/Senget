use std::process::Command;

use {
    std::{env, io},
    winres::WindowsResource,
};

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            .set_icon("assets/senget-icon.ico")
            .compile()?;
    }

    if cfg!(debug_assertions) || env::var("BUILD_SETUP").unwrap_or("false".to_owned()) == "false" {
        return Ok(());
    };
    println!("Building setup");
    let mut command = Command::new("iscc");
    command.args(["/Q", "setup.iss"]);
    command.status()?;
    Ok(())
}
