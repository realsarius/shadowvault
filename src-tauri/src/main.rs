#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // GUI modu koşulları:
    // 1. macOS .app bundle'dan açılınca --psn_* gelir
    // 2. npm run tauri dev → SHADOWVAULT_GUI=1 env var set edilir
    let gui_mode = args[1..].iter().any(|a| a.starts_with("--psn"))
        || std::env::var("SHADOWVAULT_GUI").is_ok();

    if gui_mode {
        shadowvault_lib::run();
        return;
    }

    // Diğer her durumda → CLI
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
