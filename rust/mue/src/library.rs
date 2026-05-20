use anyhow::Result;
use libmue::config::AppConfig;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn migrate_store() -> Result<()> {
    let config = AppConfig::load();
    let store_path = config.get_store_path();
    
    let mut target_dirs = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        target_dirs.push(cwd);
    }
    
    if let Some(lib) = config.library {
        if let Some(flac) = lib.flac
            && let Some(root) = flac.root {
            target_dirs.push(PathBuf::from(root));
        }
        if let Some(opus) = lib.opus
            && let Some(root) = opus.root {
            target_dirs.push(PathBuf::from(root));
        }
    }

    for dir in target_dirs {
        let expanded = libmue::utils::expand_path(&dir.to_string_lossy());
        if !expanded.exists() { continue; }
        
        for entry in WalkDir::new(&expanded).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if fs::symlink_metadata(path).map(|m| m.is_symlink()).unwrap_or(false)
                && let Ok(target) = fs::read_link(path) {
                let target_str = target.to_string_lossy();
                if let Some(idx) = target_str.find("/nix/store/") {
                    let suffix = &target_str[idx + 11..];
                    let physical_target = store_path.join("nix/store").join(suffix);
                    fs::remove_file(path)?;
                    std::os::unix::fs::symlink(physical_target, path)?;
                }
            }
        }
    }
    Ok(())
}

pub fn rebuild() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut albums = Vec::new();
    
    for entry in WalkDir::new(&cwd).into_iter().filter_map(Result::ok) {
        if entry.file_name() == "album.nix"
            && let Some(parent) = entry.path().parent() {
            albums.push(parent.to_path_buf());
        }
    }
    
    for album_dir in albums {
        let path_str = album_dir.to_string_lossy().to_string();
        let _ = crate::build::run(&path_str, None, None);
    }
    
    migrate_store()?;
    Ok(())
}

pub fn init(path: &str) -> Result<()> {
    let target = libmue::utils::expand_path(path);
    fs::create_dir_all(&target)?;
    
    let _ = std::process::Command::new("git")
        .arg("init")
        .current_dir(&target)
        .status();
        
    Ok(())
}
