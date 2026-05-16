use std::path::Path;

const TLDRAW_SKETCHES: &str = include_str!("../resources/context/tldraw-sketches.md");

pub fn ensure_context_files(project_dir: &Path) -> Result<(), std::io::Error> {
    let context_dir = project_dir.join(".myco").join("context");
    std::fs::create_dir_all(&context_dir)?;

    let sketches_path = context_dir.join("tldraw-sketches.md");
    if !sketches_path.exists() {
        std::fs::write(&sketches_path, TLDRAW_SKETCHES)?;
    }

    Ok(())
}
