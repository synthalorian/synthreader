use std::path::Path;
use crate::{BookMetadata, TocEntry};

pub struct EpubBook {
    pub metadata: BookMetadata,
    pub toc: Vec<TocEntry>,
    pub spine: Vec<SpineEntry>,
    pub cover_data: Option<Vec<u8>>,
}

pub struct SpineEntry {
    pub id: String,
    pub path: String,
    pub title: Option<String>,
}

pub fn parse_epub(path: &Path) -> anyhow::Result<EpubBook> {
    let mut doc = epub::doc::EpubDoc::new(path)?;

    let title = doc
        .mdata("title")
        .map(|item| item.value.clone())
        .unwrap_or_default();
    let authors = doc
        .mdata("creator")
        .map(|item| {
            item.value
                .split(&[',', '&'])
                .map(|a| a.trim().to_string())
                .collect()
        })
        .unwrap_or_default();

    let description = doc.mdata("description").map(|item| item.value.clone());
    let publisher = doc.mdata("publisher").map(|item| item.value.clone());
    let language = doc.mdata("language").map(|item| item.value.clone());
    let isbn_13 = doc
        .mdata("ISBN")
        .or_else(|| doc.mdata("identifier"))
        .map(|item| item.value.clone());

    let cover_data = doc.get_cover().map(|(data, _)| data);

    let toc = build_toc(&doc);
    let spine = build_spine(&doc);

    Ok(EpubBook {
        metadata: BookMetadata {
            title,
            subtitle: doc.mdata("subtitle").map(|item| item.value.clone()),
            authors,
            description,
            publisher,
            published_date: doc.mdata("date").map(|item| item.value.clone()),
            language,
            isbn_10: None,
            isbn_13,
            page_count: doc.mdata("pages").and_then(|item| item.value.parse().ok()),
            cover_data: cover_data.clone(),
            cover_url: None,
        },
        toc,
        spine,
        cover_data,
    })
}

pub fn get_chapter_content(path: &Path, href: &str) -> anyhow::Result<String> {
    let mut doc = epub::doc::EpubDoc::new(path)?;

    // href might be a fragment like "chapter1.html" or "chapter1.html#section2"
    let (resource_path, _fragment) = href.split_once('#').unwrap_or((href, ""));

    // Find the resource by path
    let content = doc
        .get_resource_by_path(resource_path)
        .or_else(|| {
            // Try matching by filename if full path doesn't work
            let filename = std::path::Path::new(resource_path)
                .file_name()?
                .to_str()?;
            let found_id = doc
                .resources
                .iter()
                .find(|(_, res)| res.path.ends_with(filename))
                .map(|(id, _)| id.clone());
            found_id.and_then(|id| doc.get_resource(&id).map(|(data, _)| data))
        })
        .ok_or_else(|| anyhow::anyhow!("Chapter not found: {}", href))?;

    let mut html = String::from_utf8_lossy(&content).to_string();

    // Get the base directory of the chapter for resolving relative image paths
    let chapter_dir = std::path::Path::new(resource_path)
        .parent()
        .and_then(|p| if p.as_os_str().is_empty() { None } else { Some(p) });

    // Find and inline images referenced with relative paths
    // Match src="relative/path.png" or src='relative/path.png'
    let img_regex = regex::Regex::new(r#"(?i)(src=["'])([^"']+\.(?:png|jpg|jpeg|gif|svg|webp))(["'])"#).unwrap();

    html = img_regex
        .replace_all(&html, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let img_path = &caps[2];
            let suffix = &caps[3];

            // Skip absolute URLs and data URIs
            if img_path.starts_with("http") || img_path.starts_with("data:") {
                return caps[0].to_string();
            }

            // Resolve relative path against chapter directory
            let resolved_path = if let Some(dir) = chapter_dir {
                dir.join(img_path).to_string_lossy().to_string()
            } else {
                img_path.to_string()
            };

            // Try to find and embed the image
            if let Some(img_data) = doc.get_resource_by_path(&resolved_path) {
                let mime = guess_mime_type(&resolved_path);
                let base64 = base64::encode(&img_data);
                format!("{}data:{};base64,{}{}", prefix, mime, base64, suffix)
            } else {
                // Try just the filename
                let filename = std::path::Path::new(img_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(img_path);

                let found = doc.resources.iter().find(|(_, res)| {
                    res.path.ends_with(filename)
                });

                if let Some((_, res)) = found {
                    let res_path = res.path.clone();
                    if let Some(img_data) = doc.get_resource_by_path(&res_path) {
                        let mime = guess_mime_type(&res_path.to_string_lossy());
                        let base64 = base64::encode(&img_data);
                        return format!("{}data:{};base64,{}{}", prefix, mime, base64, suffix);
                    }
                }

                // Leave original if we can't find it
                caps[0].to_string()
            }
        })
        .to_string();

    Ok(html)
}

fn guess_mime_type(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}

fn build_toc<R: std::io::Read + std::io::Seek>(doc: &epub::doc::EpubDoc<R>) -> Vec<TocEntry> {
    doc.toc
        .iter()
        .map(|item| TocEntry {
            label: item.label.clone(),
            href: item.content.to_string_lossy().to_string(),
            children: item
                .children
                .iter()
                .map(|child| TocEntry {
                    label: child.label.clone(),
                    href: child.content.to_string_lossy().to_string(),
                    children: Vec::new(),
                })
                .collect(),
        })
        .collect()
}

fn build_spine<R: std::io::Read + std::io::Seek>(doc: &epub::doc::EpubDoc<R>) -> Vec<SpineEntry> {
    doc.spine
        .iter()
        .filter(|item| item.linear) // Only linear reading order items
        .filter_map(|item| {
            let resource = doc.resources.get(&item.idref)?;
            Some(SpineEntry {
                id: item.idref.clone(),
                path: resource.path.to_string_lossy().to_string(),
                title: None, // We could try to extract titles from the content
            })
        })
        .collect()
}

#[cfg(test)]
mod epub_tests {
    use std::path::Path;

    #[test]
    fn test_parse_programming_ruby() {
        let path = Path::new("/home/synth/Documents/Books/programming-ruby-4_B2.0.epub");
        let epub = crate::formats::epub::parse_epub(path).unwrap();
        println!("Title: {}", epub.metadata.title);
        println!("TOC count: {}", epub.toc.len());
        for entry in &epub.toc {
            println!("  {} -> {}", entry.label, entry.href);
        }
        assert!(!epub.toc.is_empty(), "TOC should not be empty");
    }

    #[test]
    fn test_load_chapter_content() {
        let path = Path::new("/home/synth/Documents/Books/programming-ruby-4_B2.0.epub");
        let epub = crate::formats::epub::parse_epub(path).unwrap();
        let first = epub.toc.first().unwrap();
        let content = crate::formats::epub::get_chapter_content(path, &first.href).unwrap();
        println!("Content length: {}", content.len());
        assert!(!content.is_empty(), "Content should not be empty");
    }

    #[test]
    fn test_spine_parsing() {
        let path = Path::new("/home/synth/Documents/Books/programming-ruby-4_B2.0.epub");
        let epub = crate::formats::epub::parse_epub(path).unwrap();
        println!("Spine entries: {}", epub.spine.len());
        assert!(
            epub.spine.len() > 1,
            "Spine should have multiple entries, got {}",
            epub.spine.len()
        );
        for (i, entry) in epub.spine.iter().take(5).enumerate() {
            println!("  {}: id={}, path={}", i, entry.id, entry.path);
        }
    }

    #[test]
    fn test_load_spine_content() {
        let path = Path::new("/home/synth/Documents/Books/programming-ruby-4_B2.0.epub");
        let epub = crate::formats::epub::parse_epub(path).unwrap();
        let first = epub.spine.get(1).unwrap(); // Skip cover
        let content = crate::formats::epub::get_chapter_content(path, &first.path).unwrap();
        println!("First content page length: {}", content.len());
        // First pages can be short (title pages, copyright) - just verify it loads
        assert!(!content.is_empty(), "Content should not be empty");
    }
}


    #[test]
    fn test_image_inlining() {
        let path = Path::new("/home/synth/Documents/Books/programming-ruby-4_B2.0.epub");
        // f_0039.xhtml has images based on our grep earlier
        let content = crate::formats::epub::get_chapter_content(path, "f_0039.xhtml").unwrap();
        println!("Content length: {}", content.len());
        // Check that at least one image was inlined
        assert!(content.contains("data:image/png;base64,"), "Images should be inlined as base64");
        println!("Images successfully inlined!");
    }
