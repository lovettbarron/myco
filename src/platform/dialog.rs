use std::path::PathBuf;

use objc2_app_kit::NSOpenPanel;
use objc2_foundation::MainThreadMarker;

/// Show a native macOS folder picker dialog. Returns the selected path, or None if cancelled.
/// Must be called from the main thread.
pub fn pick_folder(mtm: MainThreadMarker) -> Option<PathBuf> {
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseDirectories(true);
    panel.setCanChooseFiles(false);
    panel.setAllowsMultipleSelection(false);

    let response = panel.runModal();
    // NSModalResponseOK = 1
    if response == 1 {
        panel.URLs().firstObject().and_then(|url| {
            url.path().map(|p| PathBuf::from(p.to_string()))
        })
    } else {
        None
    }
}
