use std::path::Path;

/// Load a bundled TLDraw asset by path. Returns (content_bytes, mime_type).
pub fn load_bundled_asset(path: &str) -> (Vec<u8>, &'static str) {
    let clean_path = path.trim_start_matches('/');

    // In debug mode, load from filesystem for hot-reload during development
    #[cfg(debug_assertions)]
    {
        let dist_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("tldraw")
            .join("dist");
        let file_path = dist_dir.join(clean_path);
        if file_path.exists() {
            let content = std::fs::read(&file_path).unwrap_or_default();
            let mime = mime_for_path(clean_path);
            return (content, mime);
        }
    }

    // Fallback: serve index.html for root or unknown paths
    let fallback = include_bytes!("../../resources/tldraw/dist/index.html");
    match clean_path {
        "index.html" | "" => (fallback.to_vec(), "text/html"),
        p => {
            // Try to load from dist directory at runtime
            let dist_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("tldraw")
                .join("dist");
            let file_path = dist_dir.join(p);
            if file_path.exists() {
                let content = std::fs::read(&file_path).unwrap_or_default();
                let mime = mime_for_path(p);
                (content, mime)
            } else {
                (fallback.to_vec(), "text/html")
            }
        }
    }
}

/// Determine MIME type from file extension.
pub fn mime_for_path(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "html" => "text/html",
        "js" => "application/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}
