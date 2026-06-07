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

    // Try name ID 4 (full font name) first, then name ID 1 (font family)
    // Use filter_map to skip entries that fail to decode (e.g. Mac platform)
    face.names()
        .into_iter()
        .filter(|name| name.name_id == 4)
        .filter_map(|name| name.to_string())
        .next()
        .or_else(|| {
            face.names()
                .into_iter()
                .filter(|name| name.name_id == 1)
                .filter_map(|name| name.to_string())
                .next()
        })
}


#[cfg(test)]
mod font_tests {
    #[test]
    fn test_discover_fonts() {
        let fonts = crate::fonts::discover_system_fonts();
        println!("Found {} fonts", fonts.len());
        for font in fonts.iter().take(5) {
            println!("  {} -> {} (family: {})", font.name, font.path, font.family);
        }
        assert!(!fonts.is_empty(), "Should find some fonts");
    }

    #[test]
    fn test_3270_font_found() {
        let fonts = crate::fonts::discover_system_fonts();
        let font3270 = fonts.iter().find(|f| f.name.to_lowercase().contains("3270") || f.family.to_lowercase().contains("3270"));
        println!("3270 font: {:?}", font3270);
        assert!(font3270.is_some(), "Should find 3270 Nerd Font");
    }
}

    #[test]
    fn test_font_base64_encoding() {
        let fonts = crate::fonts::discover_system_fonts();
        let font = fonts.iter().find(|f| f.name.to_lowercase().contains("3270")).expect("Should find 3270 font");
        
        let data = std::fs::read(&font.path).expect("Should read font file");
        println!("Font file size: {} bytes", data.len());
        
        let base64 = base64::encode(&data);
        println!("Base64 length: {} chars", base64.len());
        println!("Base64 prefix: {}", &base64[..100.min(base64.len())]);
        
        assert!(base64.len() > 1000, "Base64 should be substantial");
    }

    #[test]
    fn test_font_family_name() {
        let font_path = std::path::PathBuf::from("/usr/share/fonts/nerd-fonts-git/TTF/3270NerdFont-Regular.ttf");
        let family = read_font_family(&font_path);
        println!("Font family from ttf_parser: {:?}", family);
        
        // Also check name ID 1 specifically
        let data = std::fs::read(&font_path).unwrap();
        let face = ttf_parser::Face::parse(&data, 0).unwrap();
        
        for name in face.names() {
            if let Some(s) = name.to_string() {
                println!("  Name ID {} (platform {:?}): {}", name.name_id, name.platform_id, s);
            }
        }
    }

    #[test]
    fn test_font_platform_ids() {
        let font_path = std::path::PathBuf::from("/usr/share/fonts/nerd-fonts-git/TTF/3270NerdFont-Regular.ttf");
        let data = std::fs::read(&font_path).unwrap();
        let face = ttf_parser::Face::parse(&data, 0).unwrap();
        
        for name in face.names() {
            let platform = match name.platform_id {
                ttf_parser::PlatformId::Unicode => "Unicode",
                ttf_parser::PlatformId::Macintosh => "Mac",
                ttf_parser::PlatformId::Windows => "Windows",
                _ => "Other",
            };
            let has_string = name.to_string().is_some();
            println!("  Name ID {} ({}): string_ok={}", name.name_id, platform, has_string);
        }
    }
