use anyhow::Result;
use libmute::utils::{eval_config_field, expand_path};
use libmute::config::AppConfig;
use std::collections::HashMap;
use std::process::Command;
use std::path::PathBuf;

pub fn run(path: &str) -> Result<()> {
    log::debug!("Starting film fetch operation");
    log::debug!("Target path: {path}");

    let config = AppConfig::load();
    let store_path = config.get_store_path();
    let mut target_dir = PathBuf::from(path);
    if !target_dir.join("film.nix").exists() && target_dir.is_file() {
        target_dir = target_dir.parent().unwrap_or(&target_dir).to_path_buf();
    }
    let film_path = target_dir.join("film.nix");
    
    let source_type_str = libmute::utils::detect_source_type(&film_path, Some(&store_path))?;
    if source_type_str.is_empty() {
        anyhow::bail!("No valid source found in film.nix");
    }
    let source = source_type_str.as_str();

    let origin_base_path = expand_path(config.origin.as_deref().unwrap_or("."));

    let res = libmute::utils::resolve_source_origin(
        &film_path,
        Some(source),
        &store_path,
        &origin_base_path,
        "films"
    )?;

    let mut envs = HashMap::new();
    log::debug!("Using origin path: {}", res.origin_path);
    envs.insert("MUTE_ORIGIN_PATH".to_string(), res.origin_path);
    envs.insert("MUTE_SOURCE_NAME".to_string(), res.internal_name);
    envs.insert("MUTE_SANITIZED_SOURCE_NAME".to_string(), res.sanitized_name);

    let cmd_field = match source {
        "torrent" => "commands.torrent.fetch",
        "web" => "commands.web.fetch",
        _ => anyhow::bail!("Unknown source: '{source}'. Expected 'torrent' or 'web'."),
    };

    let final_cmd = eval_config_field(&film_path, cmd_field, Some(&envs), Some(&store_path))?;

    if final_cmd.is_empty() {
        anyhow::bail!("Command '{cmd_field}' is missing or empty in config.nix");
    }

    log::debug!("Final resolved fetch command: {final_cmd}");
    
    let env_bin = store_path.join("gcroots/profiles/env/bin");
    let injected_path = format!("{}:{}", env_bin.display(), std::env::var("PATH").unwrap_or_default());

    log::info!("Executing fetch command...");
    let mut cmd = Command::new("sh");
    cmd.env("PATH", injected_path).envs(&envs).arg("-c").arg(&final_cmd);
    let status = cmd.status()?;
    
    if !status.success() {
        anyhow::bail!("Command failed with status: {status}");
    }

    log::info!("Fetch operation completed successfully");

    Ok(())
}
