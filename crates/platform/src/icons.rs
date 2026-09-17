//! Loading a local icon file for display in the UI.
//!
//! Files are read from disk only (never fetched), capped in size, and their
//! type is decided by content sniffing rather than by extension, so the
//! webview only ever receives bytes that really are an image.

use std::path::Path;

use crate::PlatformError;

/// Refuse files larger than this. Browser icons are a few tens of KiB.
pub const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;

/// Decoded-enough icon: bytes plus a MIME type the webview understands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// `image/png`, `image/svg+xml` or `image/x-icon`.
    pub mime: &'static str,
    /// Raw file contents (PNG converted from ICNS on macOS).
    pub bytes: Vec<u8>,
}

/// Read `path` and check that it is a supported image.
pub fn load_icon(path: &Path) -> Result<Icon, PlatformError> {
    if !path.is_absolute() {
        return Err(PlatformError::Icon("path is not absolute".into()));
    }
    let meta = std::fs::metadata(path).map_err(|e| PlatformError::Icon(e.to_string()))?;
    if !meta.is_file() {
        return Err(PlatformError::Icon("not a regular file".into()));
    }
    if meta.len() > MAX_ICON_BYTES {
        return Err(PlatformError::Icon(format!(
            "file larger than {MAX_ICON_BYTES} bytes"
        )));
    }
    let bytes = std::fs::read(path).map_err(|e| PlatformError::Icon(e.to_string()))?;
    classify(bytes)
}

/// Decide the image type from the leading bytes.
fn classify(bytes: Vec<u8>) -> Result<Icon, PlatformError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(Icon {
            mime: "image/png",
            bytes,
        });
    }
    if bytes.starts_with(&[0, 0, 1, 0]) {
        return Ok(Icon {
            mime: "image/x-icon",
            bytes,
        });
    }
    if looks_like_svg(&bytes) {
        return Ok(Icon {
            mime: "image/svg+xml",
            bytes,
        });
    }
    #[cfg(target_os = "macos")]
    if bytes.starts_with(b"icns") {
        return icns_to_png(&bytes);
    }
    Err(PlatformError::Icon(
        "unsupported image format (use PNG, SVG or ICO)".into(),
    ))
}

/// SVG has no magic number; accept a document whose first non-blank bytes
/// (after an optional BOM) are an XML declaration or an `<svg` tag.
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let head: Vec<u8> = head
        .iter()
        .copied()
        .skip_while(u8::is_ascii_whitespace)
        .take(256)
        .collect();
    let text = String::from_utf8_lossy(&head).to_ascii_lowercase();
    text.starts_with("<svg") || (text.starts_with("<?xml") && text.contains("<svg"))
}

#[cfg(target_os = "macos")]
fn icns_to_png(bytes: &[u8]) -> Result<Icon, PlatformError> {
    let family = icns::IconFamily::read(bytes).map_err(|e| PlatformError::Icon(e.to_string()))?;
    let preferred = [
        icns::IconType::RGBA32_128x128,
        icns::IconType::RGBA32_256x256,
        icns::IconType::RGBA32_64x64,
        icns::IconType::RGBA32_32x32,
    ];
    let image = preferred
        .iter()
        .find_map(|t| family.get_icon_with_type(*t).ok())
        .or_else(|| {
            family
                .available_icons()
                .first()
                .and_then(|t| family.get_icon_with_type(*t).ok())
        })
        .ok_or_else(|| PlatformError::Icon("icns file has no usable image".into()))?;
    let mut png = Vec::new();
    image
        .write_png(&mut png)
        .map_err(|e| PlatformError::Icon(e.to_string()))?;
    Ok(Icon {
        mime: "image/png",
        bytes: png,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_content_not_extension() {
        assert_eq!(
            classify(b"\x89PNG\r\n\x1a\nrest".to_vec()).unwrap().mime,
            "image/png"
        );
        assert_eq!(
            classify(b"\x00\x00\x01\x00rest".to_vec()).unwrap().mime,
            "image/x-icon"
        );
        assert_eq!(
            classify(b"  <svg xmlns='x'/>".to_vec()).unwrap().mime,
            "image/svg+xml"
        );
        assert_eq!(
            classify(b"\xef\xbb\xbf<?xml version='1.0'?>\n<svg/>".to_vec())
                .unwrap()
                .mime,
            "image/svg+xml"
        );
        assert!(classify(b"<html><svg/></html>".to_vec()).is_err());
        assert!(classify(b"GIF89a".to_vec()).is_err());
        assert!(classify(Vec::new()).is_err());
    }

    #[test]
    fn load_icon_enforces_path_rules() {
        assert!(load_icon(Path::new("relative.png")).is_err());
        let dir = tempfile::tempdir().unwrap();
        assert!(load_icon(dir.path()).is_err(), "directories are refused");
        let file = dir.path().join("x.png");
        std::fs::write(&file, b"\x89PNG\r\n\x1a\n").unwrap();
        assert_eq!(load_icon(&file).unwrap().mime, "image/png");
        std::fs::write(&file, b"not an image").unwrap();
        assert!(load_icon(&file).is_err());
    }
}
