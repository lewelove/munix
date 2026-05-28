use anyhow::Result;
use libmute::utils::{eval_nix_field, eval_config_field, expand_path};
use libmute::config::AppConfig;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;

pub fn run(path: &str) -> Result<()> {
    log::debug!("Starting mute build for film at path: {path}");

    let config = AppConfig::load();
    let store_path = config.get_store_path();
    
    let target_dir = Path::new(path).canonicalize().unwrap_or_else(|_| PathBuf::from(path));
    let target_path = target_dir.join("film.nix");
    
    if !target_path.exists() {
        anyhow::bail!("film.nix not found in {}", target_dir.display());
    }

    let source_type_str = libmute::utils::detect_source_type(&target_path, Some(&store_path))?;
    let source_type = if source_type_str.is_empty() { None } else { Some(source_type_str.as_str()) };

    if source_type == Some("torrent") {
        verify_torrent_file(&target_path, &target_dir, &store_path)?;
    }

    let origin_base_path = expand_path(config.origin.as_deref().unwrap_or("."));
    let res = libmute::utils::resolve_source_origin(
        &target_path,
        source_type,
        &store_path,
        &origin_base_path,
        "films"
    )?;

    let mut envs = HashMap::new();
    envs.insert("MUTE_ORIGIN_PATH".to_string(), res.origin_path.clone());
    envs.insert("MUTE_SOURCE_NAME".to_string(), res.internal_name.clone());
    envs.insert("MUTE_SANITIZED_SOURCE_NAME".to_string(), res.sanitized_name.clone());

    if source_type == Some("torrent") {
        let env_bin = store_path.join("gcroots/profiles/env/bin");
        let injected_path = format!("{}:{}", env_bin.display(), std::env::var("PATH").unwrap_or_default());
        execute_verify_command(&target_path, &store_path, &envs, &injected_path)?;
    }

    validate_poster(&target_path, &target_dir, &store_path)?;
    create_video_symlink(&target_path, &target_dir, &res.origin_path, &store_path)?;

    log::info!("Film build completed successfully.");
    Ok(())
}

fn verify_torrent_file(target_path: &Path, target_dir: &Path, store_path: &Path) -> Result<()> {
    log::debug!("Source type is torrent, initiating pre-verify sequence");
    let torrent_file_raw = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "source.torrent.file", None, Some(store_path))?;
    let torrent_hash = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "source.torrent.hash", None, Some(store_path))?;
    
    if !torrent_file_raw.is_empty() {
        let torrent_path = if torrent_file_raw.starts_with("./") {
            target_dir.join(torrent_file_raw.trim_start_matches("./"))
        } else {
            PathBuf::from(&torrent_file_raw)
        };
        log::debug!("Resolved torrent file path: {}", torrent_path.display());

        if torrent_path.exists() {
            let actual_torrent_hash = libmute::utils::get_file_hash(&torrent_path, Some(store_path))?;
            libmute::utils::check_hash(&actual_torrent_hash, &torrent_hash, "source.torrent.hash")?;
        } else {
            anyhow::bail!("Torrent file not found at {}", torrent_path.display());
        }
    }
    Ok(())
}

fn execute_verify_command(target_path: &Path, store_path: &Path, envs: &HashMap<String, String>, injected_path: &str) -> Result<()> {
    let verify_cmd = eval_config_field(target_path, "commands.torrent.verify", Some(envs), Some(store_path))?;
    if !verify_cmd.is_empty() {
        log::info!("Executing torrent verification command");
        log::debug!("Verify command: {verify_cmd}");
        let status = Command::new("sh")
            .env("PATH", injected_path)
            .envs(envs)
            .arg("-c")
            .arg(&verify_cmd)
            .status()?;
        if !status.success() {
            anyhow::bail!("Torrent verification failed. Logic returned non-zero exit code.");
        }
    }
    Ok(())
}

fn validate_poster(target_path: &Path, target_dir: &Path, store_path: &Path) -> Result<()> {
    let poster_file_raw = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "poster.file", None, Some(store_path))?;
    let poster_hash = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "poster.hash", None, Some(store_path))?;
    if !poster_file_raw.is_empty() && poster_file_raw != "null" {
        let poster_path = if poster_file_raw.starts_with("./") {
            target_dir.join(poster_file_raw.trim_start_matches("./"))
        } else {
            PathBuf::from(&poster_file_raw)
        };
        log::debug!("Validating poster file: {}", poster_path.display());
        if poster_path.exists() {
            let actual_poster_hash = libmute::utils::get_file_hash(&poster_path, Some(store_path))?;
            libmute::utils::check_hash(&actual_poster_hash, &poster_hash, "poster.hash")?;
        } else {
            log::warn!("Poster file not found at {}", poster_path.display());
        }
    }
    Ok(())
}

fn create_video_symlink(target_path: &Path, target_dir: &Path, origin_path: &str, store_path: &Path) -> Result<()> {
    let video_rel = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "film.video", None, Some(store_path))?;
    if video_rel.is_empty() {
        anyhow::bail!("film.video is empty in film.nix");
    }

    let source_video = PathBuf::from(origin_path).join(&video_rel);
    if !source_video.exists() {
        anyhow::bail!("Target video file not found at origin: {}", source_video.display());
    }

    let title = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "film.metadata.title", None, Some(store_path))?;
    let year = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "film.metadata.year", None, Some(store_path))?;
    let director = eval_nix_field::<std::collections::hash_map::RandomState>(target_path, "film.metadata.director", None, Some(store_path))?;

    let ext = source_video.extension().and_then(|e| e.to_str()).unwrap_or("mkv");

    let t = if title.is_empty() { "Unknown Title" } else { &title };
    let y = if year.is_empty() { "Unknown Year" } else { &year };
    let d = if director.is_empty() { "Unknown Director" } else { &director };

    let link_name = format!("{t} - {y} - {d}.{ext}").replace('/', "_");
    let link_target = target_dir.join(&link_name);

    if let Ok(entries) = fs::read_dir(target_dir) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if let Ok(m) = fs::symlink_metadata(&p) && m.is_symlink() {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                if name.ends_with(&format!(".{ext}")) || p == link_target {
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }

    log::info!("Creating local symlink: {}", link_target.display());
    std::os::unix::fs::symlink(&source_video, &link_target)?;
    Ok(())
}
