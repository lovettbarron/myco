//! IPC contract tests for CanvasManager::handle_ipc_message.
//!
//! These tests verify Rust-webview message round-trips without launching a webview.
//! They test the message handling contract: save writes files, shortcuts return false,
//! unknown types are handled gracefully, malformed JSON doesn't panic, and oversized
//! saves are rejected.

use myco::canvas::{CanvasManager, CanvasState};
use myco::grid::PanelId;
use std::fs;

#[test]
fn test_save_message_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test_canvas.excalidraw");
    let mut manager = CanvasManager::new(dir.path().to_path_buf());

    let panel_id = PanelId(42);
    let state = CanvasState::new("test_canvas".to_string(), file_path.clone());
    manager.insert_canvas_state(panel_id, state);

    let msg = r#"{"type":"save","data":{"elements":[{"id":"s1","type":"rectangle"}]}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);

    assert!(changed, "save message should return true (state changed)");
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("elements"), "file should contain elements data");
    assert!(content.contains("s1"), "file should contain element id");
}

#[test]
fn test_shortcut_message_no_state_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(1);
    let state = CanvasState::new("canvas1".to_string(), dir.path().join("c1.excalidraw"));
    manager.insert_canvas_state(panel_id, state);

    let msg = r#"{"type":"shortcut","data":{"key":"cmd+z"}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);
    assert!(!changed, "shortcut message should return false");
}

#[test]
fn test_unknown_message_type() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(1);
    let state = CanvasState::new("canvas1".to_string(), dir.path().join("c1.excalidraw"));
    manager.insert_canvas_state(panel_id, state);

    let msg = r#"{"type":"unknown_action","data":{}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);
    assert!(!changed, "unknown message type should return false");
}

#[test]
fn test_malformed_json_no_panic() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(1);

    let msg = "this is not json at all {{{";
    let changed = manager.handle_ipc_message(&panel_id, msg);
    assert!(!changed, "malformed JSON should return false without panic");
}

#[test]
fn test_save_nonexistent_panel() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(999);

    let msg = r#"{"type":"save","data":{"elements":[]}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);
    assert!(changed, "save message type should be recognized even without panel state");
    let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "no files should be written for nonexistent panel"
    );
}

#[test]
fn test_oversized_save_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("big.excalidraw");
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(1);
    let state = CanvasState::new("big".to_string(), file_path.clone());
    manager.insert_canvas_state(panel_id, state);

    let big_data = "x".repeat(51 * 1024 * 1024);
    let msg = format!(r#"{{"type":"save","data":"{}"}}"#, big_data);
    let changed = manager.handle_ipc_message(&panel_id, &msg);

    assert!(changed, "save message type should be recognized");
    assert!(!file_path.exists(), "oversized save should not write file");
}
