use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemFont {
    pub name: String,
    pub path: String,
    pub family: String,
}

pub fn discover_system_fonts() -> Vec<SystemFont> {
    let mut fonts = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    let font_dirs = get_font_directories();

    for dir in font_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            scan_font_directory(entries, &mut fonts, &mut seen_paths);
        }
    }

    // Sort by name for consistent ordering
    fonts.sort_by_key(|a| a.name.to_lowercase());
    fonts
}

fn get_font_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Linux system fonts
    dirs.push(PathBuf::from("/usr/share/fonts"));
    dirs.push(PathBuf::from("/usr/local/share/fonts"));

    // User fonts
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/fonts"));
        dirs.push(home.join(".fonts"));
        dirs.push(home.join("Library/Fonts")); // macOS
    }

    // macOS system fonts
    dirs.push(PathBuf::from("/Library/Fonts"));
    dirs.push(PathBuf::from("/System/Library/Fonts"));

    dirs
}

fn scan_font_directory(
    entries: std::fs::ReadDir,
    fonts: &mut Vec<SystemFont>,
    seen_paths: &mut std::collections::HashSet<PathBuf>,
) {
    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                scan_font_directory(sub_entries, fonts, seen_paths);
            }
            continue;
        }

        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if matches!(ext.as_str(), "ttf" | "otf" | "woff" | "woff2")
                && seen_paths.insert(path.clone())
                && let Some(font) = font_from_path(&path)
            {
                fonts.push(font);
            }
        }
    }
}

fn font_from_path(path: &PathBuf) -> Option<SystemFont> {
    let path_str = path.to_string_lossy().to_string();

    // Try to extract font family from filename
    let filename = path
        .file_stem()?
        .to_string_lossy()
        .to_string();

    // Try to read font family from the font file itself using ttf-parser
    let family = read_font_family(path).unwrap_or_else(|| filename.clone());

    let display_name = sanitize_font_name(&filename);

    Some(SystemFont {
        name: display_name,
        path: path_str,
        family,
    })
}

fn sanitize_font_name(name: &str) -> String {
    name.replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_font_family(path: &PathBuf) -> Option<String> {
    let data = std::fs::read(path).ok()?;

    let face = ttf_parser::Face::parse(&data, 0).ok()?;

    face.names()
        .into_iter()
        .find(|name| name.name_id == 4) // Full font name
        .and_then(|name| name.to_string())
        .or_else(|| {
            face.names()
                .into_iter()
                .find(|name| name.name_id == 1) // Font family
                .and_then(|name| name.to_string())
        })
}
