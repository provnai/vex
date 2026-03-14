/// Tiered discovery for the Magpie compiler binary.
/// 1. Check MAGPIE_BIN_PATH environment variable.
/// 2. Check relative path ../magpie/target/release/magpie(.exe) for sibling repository layout.
/// 3. Default to "magpie" for system PATH lookup.
pub fn find_magpie_binary() -> String {
    // Priority 1: Environment Variable
    if let Ok(path) = std::env::var("MAGPIE_BIN_PATH") {
        return path;
    }

    // Priority 2: Relative Discovery (Sibling Repository)
    if let Ok(cwd) = std::env::current_dir() {
        // Search up for workspace root (containing Cargo.lock)
        let mut current = Some(cwd.as_path());
        while let Some(path) = current {
            if path.join("Cargo.lock").exists() {
                // Found workspace root, look for sibling 'magpie'
                if let Some(parent) = path.parent() {
                    let magpie_dir = parent.join("magpie").join("target").join("release");
                    let bin = magpie_dir.join("magpie");
                    let exe = magpie_dir.join("magpie.exe");

                    if bin.exists() {
                        return bin.to_string_lossy().to_string();
                    }
                    if exe.exists() {
                        return exe.to_string_lossy().to_string();
                    }
                }
                break;
            }
            current = path.parent();
        }
    }

    // Priority 3: System PATH Fallback
    "magpie".to_string()
}
