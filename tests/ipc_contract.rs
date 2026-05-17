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
fn test_save_message_writes_tldr() {
    let dir = tempfile::tempdir().unwrap();
    let tldr_path = dir.path().join("test_canvas.tldr");
    let mut manager = CanvasManager::new(dir.path().to_path_buf());

    let panel_id = PanelId(42);
    let state = CanvasState::new("test_canvas".to_string(), tldr_path.clone());
    manager.insert_canvas_state(panel_id, state);

    let msg = r#"{"type":"save","data":{"shapes":[{"id":"s1","type":"draw"}]}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);

    assert!(changed, "save message should return true (state changed)");
    let content = fs::read_to_string(&tldr_path).unwrap();
    assert!(content.contains("shapes"), "file should contain shapes data");
    assert!(content.contains("s1"), "file should contain shape id");
}

#[test]
fn test_shortcut_message_no_state_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(1);
    let state = CanvasState::new("canvas1".to_string(), dir.path().join("c1.tldr"));
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
    let state = CanvasState::new("canvas1".to_string(), dir.path().join("c1.tldr"));
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
    // Don't insert any canvas state
    let panel_id = PanelId(999);

    let msg = r#"{"type":"save","data":{"shapes":[]}}"#;
    let changed = manager.handle_ipc_message(&panel_id, msg);
    // Returns true (message recognized as "save") but no file written
    assert!(changed, "save message type should be recognized even without panel state");
    // No files should exist in dir
    let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "no files should be written for nonexistent panel"
    );
}

#[test]
fn test_oversized_save_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let tldr_path = dir.path().join("big.tldr");
    let mut manager = CanvasManager::new(dir.path().to_path_buf());
    let panel_id = PanelId(1);
    let state = CanvasState::new("big".to_string(), tldr_path.clone());
    manager.insert_canvas_state(panel_id, state);

    // Create a save message with data that serializes to > 50MB
    let big_data = "x".repeat(51 * 1024 * 1024);
    let msg = format!(r#"{{"type":"save","data":"{}"}}"#, big_data);
    let changed = manager.handle_ipc_message(&panel_id, &msg);

    assert!(changed, "save message type should be recognized");
    assert!(!tldr_path.exists(), "oversized save should not write file");
}
