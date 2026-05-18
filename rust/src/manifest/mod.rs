use anyhow::Result;
use globset::{Glob, GlobSetBuilder};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::config::AppConfig;

fn build_globset(tracks_filter: &str) -> Result<globset::GlobSet> {
    log::debug!("Building globset with filter: {}", tracks_filter);
    
    let mut builder = GlobSetBuilder::new();
    for part in tracks_filter.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let pattern = if !trimmed.contains('/') && !trimmed.contains('*') && !trimmed.contains('?') {
            format!("**/*.{}", trimmed.trim_start_matches('.'))
        } else {
            trimmed.to_string()
        };
        
        log::debug!("Adding pattern to globset: {}", pattern);
        
        builder.add(Glob::new(&pattern)?);
    }
    Ok(builder.build()?)
}

pub fn run(path: &str, tracks_filter: &str) -> Result<()> {
    log::debug!("Starting manifest generation");
    log::debug!("Target path: {}", path);

    let config = AppConfig::load();
    let store_path = config.get_store_path();

    let target_path = Path::new(path).canonicalize().unwrap_or_else(|_| PathBuf::from(path));
    let dir = if target_path.is_file() {
        target_path.parent().unwrap().to_path_buf()
    } else {
        target_path
    };

    log::debug!("Resolved directory to scan: {}", dir.display());

    let globset = build_globset(tracks_filter)?;
    let mut files = Vec::new();

    for entry in WalkDir::new(&dir).max_depth(3).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|e| e.to_str())
            && (ext.eq_ignore_ascii_case("flac") || ext.eq_ignore_ascii_case("mp3") || ext.eq_ignore_ascii_case("wav"))
        {
            let rel_path = entry.path().strip_prefix(&dir).unwrap_or(entry.path());
            let path_str = rel_path.to_string_lossy();
            if globset.is_match(path_str.as_ref()) {
                log::debug!("Matched track file: {}", path_str);
                files.push(entry.path().to_path_buf());
            }
        }
    }
    
    files.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));

    let mut cover_file = "./cover.png".to_string();
    let mut cover_hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string();

    if dir.join("cover.png").exists() {
        log::debug!("Found cover.png");
        if let Ok(h) = crate::utils::get_file_hash(&dir.join("cover.png"), Some(&store_path)) {
            cover_hash = h;
        }
    } else if dir.join("cover.jpg").exists() {
        log::debug!("Found cover.jpg");
        cover_file = "./cover.jpg".to_string();
        if let Ok(h) = crate::utils::get_file_hash(&dir.join("cover.jpg"), Some(&store_path)) {
            cover_hash = h;
        }
    } else {
        log::debug!("No cover image found. Using placeholders.");
    }

    let mut torrent_file = "./Info/source.torrent".to_string();
    let mut torrent_hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string();

    let mut torrent_found = false;
    if let Ok(entries) = std::fs::read_dir(dir.join("Info")) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("torrent") {
                torrent_file = format!("./Info/{}", p.file_name().unwrap().to_string_lossy());
                log::debug!("Found torrent file at: {}", torrent_file);
                if let Ok(h) = crate::utils::get_file_hash(&p, Some(&store_path)) {
                    torrent_hash = h;
                }
                torrent_found = true;
                break;
            }
        }
    }
    
    if !torrent_found
        && let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) == Some("torrent") {
                    torrent_file = format!("./{}", p.file_name().unwrap().to_string_lossy());
                    log::debug!("Found torrent file at: {}", torrent_file);
                    if let Ok(h) = crate::utils::get_file_hash(&p, Some(&store_path)) {
                        torrent_hash = h;
                    }
                    break;
                }
            }
        }

    log::debug!("Generating manifest output");

    let mut out = String::new();
    let _ = writeln!(out, "{{ munix }}:");
    let _ = writeln!(out, "munix.mkAlbum {{");
    let _ = writeln!(out, "  name = \"\";");
    let _ = writeln!(out, "  origin = {{");
    let _ = writeln!(out, "    path = \"\";");
    let _ = writeln!(out, "    hash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\";");
    let _ = writeln!(out, "  }};");
    let _ = writeln!(out, "  source.torrent = {{");
    let _ = writeln!(out, "    file = {torrent_file};");
    let _ = writeln!(out, "    name = \"\";");
    let _ = writeln!(out, "    hash = \"{torrent_hash}\";");
    let _ = writeln!(out, "  }};");
    let _ = writeln!(out, "  source.web = {{");
    let _ = writeln!(out, "    url = \"\";");
    let _ = writeln!(out, "    hash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\";");
    let _ = writeln!(out, "  }};");
    let _ = writeln!(out, "  cover = {{");
    let _ = writeln!(out, "    file = {cover_file};");
    let _ = writeln!(out, "    hash = \"{cover_hash}\";");
    let _ = writeln!(out, "  }};");
    let _ = writeln!(out, "  album = {{");
    let _ = writeln!(out, "    metadata = {{");
    let _ = writeln!(out, "    }};");
    let _ = writeln!(out, "    mbid = {{");
    let _ = writeln!(out, "    }};");
    let _ = writeln!(out, "  }};");

    let _ = writeln!(out, "  tracks = [");
    
    if files.is_empty() {
        let _ = writeln!(out, "    {{");
        let _ = writeln!(out, "      file = \"\";");
        let _ = writeln!(out, "      metadata = {{");
        let _ = writeln!(out, "      }};");
        let _ = writeln!(out, "      mbid = {{");
        let _ = writeln!(out, "      }};");
        let _ = writeln!(out, "    }}");
    } else {
        for f in &files {
            let rel = f.strip_prefix(&dir).unwrap_or(f).to_string_lossy();
            let _ = writeln!(out, "    {{");
            let _ = writeln!(out, "      file = \"{rel}\";");
            let _ = writeln!(out, "      metadata = {{");
            let _ = writeln!(out, "      }};");
            let _ = writeln!(out, "      mbid = {{");
            let _ = writeln!(out, "      }};");
            let _ = writeln!(out, "    }}");
        }
    }

    let _ = writeln!(out, "  ];");
    let _ = writeln!(out, "}}");

    println!("{out}");

    Ok(())
}
