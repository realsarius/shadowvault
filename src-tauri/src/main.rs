#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let has_no_cli_args = args.len() <= 1;
    let gui_mode = has_no_cli_args
        || args[1..].iter().any(|a| a.starts_with("--psn"))
        || std::env::var("SHADOWVAULT_GUI").is_ok();

    if gui_mode {
        shadowvault_lib::run();
        return;
    }

    match shadowvault_lib::cli::Cli::try_parse() {
        Ok(cli) => {
            std::process::exit(shadowvault_lib::cli::run(cli));
        }
        Err(e) => {
            e.print().ok();
            std::process::exit(2);
        }
    }
}
