use std::path::Path;
use crate::{BookMetadata, TocEntry};

pub struct EpubBook {
    pub metadata: BookMetadata,
    pub toc: Vec<TocEntry>,
    pub cover_data: Option<Vec<u8>>,
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
        cover_data,
    })
}

pub fn get_chapter_content(path: &Path, href: &str) -> anyhow::Result<String> {
    let mut doc = epub::doc::EpubDoc::new(path)?;
    
    // href might be a fragment like "chapter1.html" or "chapter1.html#section2"
    let (resource_path, _fragment) = href.split_once('#').unwrap_or((href, ""));
    
    // Find the resource by path
    let content = doc.get_resource_by_path(resource_path)
        .or_else(|| {
            // Try matching by filename if full path doesn't work
            let filename = std::path::Path::new(resource_path)
                .file_name()?
                .to_str()?;
            let found_id = doc.resources.iter()
                .find(|(_, res)| res.path.ends_with(filename))
                .map(|(id, _)| id.clone());
            found_id.and_then(|id| doc.get_resource(&id).map(|(data, _)| data))
        })
        .ok_or_else(|| anyhow::anyhow!("Chapter not found: {}", href))?;

    let html = String::from_utf8_lossy(&content).to_string();
    Ok(html)
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
