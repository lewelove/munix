use anyhow::Result;
use walkdir::WalkDir;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut films = Vec::new();
    
    for entry in WalkDir::new(&cwd).into_iter().filter_map(Result::ok) {
        if entry.file_name() == "film.nix"
            && let Some(parent) = entry.path().parent() {
            films.push(parent.to_path_buf());
        }
    }
    
    for film_dir in films {
        let path_str = film_dir.to_string_lossy().to_string();
        let _ = crate::film::build::run(&path_str);
    }
    
    Ok(())
}
