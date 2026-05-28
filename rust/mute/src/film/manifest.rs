use anyhow::Result;
use lava_torrent::torrent::v1::Torrent;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use serde_json::json;

pub fn run(path_str: &str, video_filter: Option<&str>, torrent_path: Option<&str>, intermediary: bool) -> Result<()> {
    let target_dir = Path::new(path_str).canonicalize().unwrap_or_else(|_| PathBuf::from(path_str));
    let film_nix_path = target_dir.join("film.nix");
    
    let origin_hash = get_origin_hash(&film_nix_path);
    let t_path = resolve_torrent_path(&target_dir, torrent_path)?;
    
    let torrent_hash = libmute::utils::get_file_hash(&t_path, None).unwrap_or_default();
    let torrent = Torrent::read_from_file(&t_path).map_err(|_| anyhow::anyhow!("Torrent parse error"))?;

    let rel_t_path = t_path.canonicalize().unwrap_or_else(|_| t_path.clone())
        .strip_prefix(&target_dir).map_or_else(|_| t_path.clone(), std::path::Path::to_path_buf);

    let (poster_file, poster_hash) = get_poster_info(&target_dir);
    let video_path = resolve_video_path(&torrent, video_filter)?;

    let data = json!({
        "name": "",
        "origin": { "hash": origin_hash },
        "source": {
            "type": "torrent",
            "torrent": {
                "file": rel_t_path.to_string_lossy(),
                "hash": torrent_hash
            },
            "web": { "url": "", "hash": "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" }
        },
        "poster": { "file": poster_file, "hash": poster_hash },
        "film": { 
            "metadata": { "title": "", "year": "", "director": "" },
            "urls": { "letterboxd": "" },
            "video": video_path
        }
    });

    if intermediary {
        println!("{}", serde_json::to_string_pretty(&data)?);
        return Ok(());
    }

    evaluate_template(&target_dir, &data)
}

fn get_origin_hash(film_nix_path: &Path) -> String {
    if film_nix_path.exists()
        && let Ok(h) = libmute::utils::eval_nix_field::<std::collections::hash_map::RandomState>(film_nix_path, "origin.hash", None, None)
        && !h.is_empty()
    {
        h
    } else {
        "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string()
    }
}

fn resolve_torrent_path(target_dir: &Path, torrent_path: Option<&str>) -> Result<PathBuf> {
    let t_path = torrent_path.map_or_else(
        || {
            let mut found = None;
            if target_dir.is_dir() && let Ok(entries) = std::fs::read_dir(target_dir) {
                for entry in entries.filter_map(Result::ok) {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("torrent") {
                        found = Some(entry.path());
                        break;
                    }
                }
            }
            found.unwrap_or_else(|| PathBuf::from("."))
        },
        |t| Path::new(t).to_path_buf(),
    );

    if !t_path.exists() {
        anyhow::bail!("Torrent file not found");
    }
    Ok(t_path)
}

fn get_poster_info(target_dir: &Path) -> (String, String) {
    let mut poster_file = "poster.png".to_string();
    let mut poster_hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string();
    
    for candidate in &["poster.png", "poster.jpg", "cover.png", "cover.jpg"] {
        let p = target_dir.join(candidate);
        if p.exists() {
            poster_file = candidate.to_string();
            if let Ok(h) = libmute::utils::get_file_hash(&p, None) {
                poster_hash = h;
            }
            break;
        }
    }
    (poster_file, poster_hash)
}

fn resolve_video_path(torrent: &Torrent, video_filter: Option<&str>) -> Result<String> {
    if let Some(files) = &torrent.files {
        let mut valid_paths = Vec::new();
        if let Some(filter) = video_filter {
            let globset = libmute::utils::build_globset(filter)?;
            for f in files {
                if globset.is_match(f.path.to_string_lossy().as_ref()) {
                    valid_paths.push(f.path.clone());
                }
            }
        } else {
            let mut largest = None;
            let mut max_len = 0;
            for f in files {
                let ext = f.path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if ["mkv", "mp4", "avi", "mov", "webm"].contains(&ext.as_str()) && f.length > max_len {
                    max_len = f.length;
                    largest = Some(f.path.clone());
                }
            }
            if let Some(l) = largest { valid_paths.push(l); }
        }
        return Ok(valid_paths.first().map_or_else(String::new, |path_buf| {
            format!("{}/{}", torrent.name, path_buf.to_string_lossy())
        }));
    }

    let is_match = if let Some(filter) = video_filter {
        libmute::utils::build_globset(filter)?.is_match(&torrent.name)
    } else {
        let ext = Path::new(&torrent.name).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        ["mkv", "mp4", "avi", "mov", "webm"].contains(&ext.as_str())
    };

    Ok(if is_match { torrent.name.clone() } else { String::new() })
}

fn evaluate_template(target_dir: &Path, data: &serde_json::Value) -> Result<()> {
    let json_str = serde_json::to_string(data)?;
    let temp_path = target_dir.join(".mute-tmp.json");
    fs::write(&temp_path, &json_str)?;
    
    let temp_path_str = temp_path.to_string_lossy();
    let flake_uri = libmute::utils::get_mute_flake_uri();
    let expr = format!("(builtins.getFlake \"{flake_uri}\").lib.filmNixTemplate {{ data = builtins.fromJSON (builtins.readFile (/. + \"{temp_path_str}\")); }}");
    
    let output = Command::new("nix").args(["eval", "--raw", "--impure", "--expr", &expr]).output();
    let _ = fs::remove_file(&temp_path);
    
    if let Ok(out) = output {
        if out.status.success() {
            println!("{}", String::from_utf8_lossy(&out.stdout).trim_end());
        } else {
            anyhow::bail!("Template generation failed:\n{}", String::from_utf8_lossy(&out.stderr));
        }
    }
    
    Ok(())
}
