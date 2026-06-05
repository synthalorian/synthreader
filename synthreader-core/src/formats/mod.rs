pub mod epub;

use std::path::Path;
use crate::{BookFormat, BookMetadata};

pub fn detect_format(path: &Path) -> Option<BookFormat> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(BookFormat::from_extension)
}

pub fn parse_book(path: &Path) -> anyhow::Result<BookMetadata> {
    match detect_format(path) {
        Some(BookFormat::Epub) => {
            let epub = epub::parse_epub(path)?;
            Ok(epub.metadata)
        }
        _ => anyhow::bail!("unsupported or undetected format"),
    }
}
