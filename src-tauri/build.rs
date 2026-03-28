fn main() {
    // Load .env for local development (values get baked into binary at compile time)
    println!("cargo:rerun-if-changed=.env");
    if let Ok(contents) = std::fs::read_to_string(".env") {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim().trim_matches('"');
                println!("cargo:rustc-env={}={}", key, val);
            }
        }
    }
    tauri_build::build()
}
