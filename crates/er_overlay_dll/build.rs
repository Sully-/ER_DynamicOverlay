use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let target_dir = env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| workspace_root.join("target").to_string_lossy().into_owned());
    let out_dir = PathBuf::from(&target_dir).join(&profile);
    if let Err(e) = fs::create_dir_all(&out_dir) {
        println!("cargo:warning=Could not create {}: {e}", out_dir.display());
    }

    let config_src = workspace_root.join("er_overlay.toml");
    println!("cargo:rerun-if-changed={}", config_src.display());
    if config_src.is_file() {
        let dest = out_dir.join("er_overlay.toml");
        if let Err(e) = fs::copy(&config_src, &dest) {
            println!(
                "cargo:warning=Could not copy {} to {}: {e}",
                config_src.display(),
                dest.display()
            );
        }
    }

    let layouts_src = workspace_root.join("layouts");
    println!("cargo:rerun-if-changed={}", layouts_src.display());
    if layouts_src.is_dir() {
        let layouts_dest = out_dir.join("layouts");
        if let Err(e) = copy_dir_all(&layouts_src, &layouts_dest) {
            println!(
                "cargo:warning=Could not copy layouts/ to {}: {e}",
                layouts_dest.display()
            );
        }
    }

    let boss_tables_src = workspace_root
        .join("crates")
        .join("er_game_state")
        .join("tables");
    println!("cargo:rerun-if-changed={}", boss_tables_src.display());
    if boss_tables_src.is_dir() {
        for entry in fs::read_dir(&boss_tables_src)
            .into_iter()
            .flatten()
            .flatten()
        {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let lang = entry.file_name().to_string_lossy().into_owned();
            let dest_dir = out_dir.join("tables").join(&lang);
            let mut dest_created = false;
            for table in ["bosses.toml", "checks.toml"] {
                let src = entry.path().join(table);
                if !src.is_file() {
                    continue;
                }
                println!("cargo:rerun-if-changed={}", src.display());
                if !dest_created {
                    if let Err(e) = fs::create_dir_all(&dest_dir) {
                        println!("cargo:warning=Could not create {}: {e}", dest_dir.display());
                        break;
                    }
                    dest_created = true;
                }
                if let Err(e) = fs::copy(&src, dest_dir.join(table)) {
                    println!(
                        "cargo:warning=Could not copy {table} to {}: {e}",
                        dest_dir.display()
                    );
                }
            }
        }
    }

    let icons_src = workspace_root.join("assets").join("icons");
    if icons_src.is_dir() {
        for entry in fs::read_dir(&icons_src).into_iter().flatten().flatten() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
        let icons_dest = out_dir.join("assets").join("icons");
        if let Err(e) = copy_dir_all(&icons_src, &icons_dest) {
            println!(
                "cargo:warning=Could not copy assets/icons/ to {}: {e}",
                icons_dest.display()
            );
        }
    }
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
