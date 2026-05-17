//! Unified toast notification system (D-14).
//!
//! Provides a centralized `ToastManager` for all toast notifications in the app,
//! replacing the settings-local `NotificationToast`. Supports intervention alerts,
//! settings conflict toasts, and general info/error toasts.
//!
//! Features:
//! - Max 3 visible toasts (D-11, T-06-03)
//! - Rate limiting: 1 per pattern per panel per 10 seconds (T-06-03)
//! - Pattern suppression per panel (D-07)
//! - Auto-dismiss after configurable duration (D-13)

pub mod renderer;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::grid::panel::PanelId;

/// Duration for intervention toasts (D-13: 8 seconds).
pub const INTERVENTION_TOAST_DURATION: Duration = Duration::from_secs(8);

/// Duration for informational toasts (settings conflicts, etc.).
pub const INFO_TOAST_DURATION: Duration = Duration::from_secs(3);

/// Maximum number of visible toasts at once (D-11, T-06-03).
pub const MAX_VISIBLE_TOASTS: usize = 3;

/// Rate limit window: max 1 toast per pattern per panel per this duration (T-06-03).
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);

/// Type of toast notification.
#[derive(Debug, Clone, PartialEq)]
pub enum ToastType {
    /// Settings conflict toast (e.g., shortcut key displaced).
    Conflict,
    /// AI intervention needed (e.g., Claude Code awaiting input).
    Intervention,
    /// General informational toast.
    Info,
    /// Error toast.
    Error,
}

/// A single toast notification.
#[derive(Debug, Clone)]
pub struct Toast {
    /// Unique identifier for this toast.
    pub id: u64,
    /// Type of toast (determines accent color).
    pub toast_type: ToastType,
    /// Primary message text.
    pub message: String,
    /// Optional attribution text (e.g., tool name).
    pub attribution: Option<String>,
    /// Optional source panel (clicking navigates there).
    pub source_panel: Option<PanelId>,
    /// Optional pattern ID (for suppression and rate limiting).
    pub pattern_id: Option<String>,
    /// Optional action link text (e.g., "Undo").
    pub action_text: Option<String>,
    /// When this toast was created.
    pub shown_at: Instant,
    /// How long before auto-dismiss.
    pub duration: Duration,
}

/// Manages toast lifecycle, rate limiting, and pattern suppression.
pub struct ToastManager {
    /// Active toasts (newest first after sort).
    toasts: Vec<Toast>,
    /// Next ID to assign.
    next_id: u64,
    /// Suppressed patterns per panel (D-07).
    suppressed: HashMap<String, HashSet<PanelId>>,
    /// Rate limit tracking: (pattern_id, panel_id) -> last shown time.
    rate_limits: HashMap<(String, PanelId), Instant>,
}

impl ToastManager {
    /// Create a new empty toast manager.
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            next_id: 1,
            suppressed: HashMap::new(),
            rate_limits: HashMap::new(),
        }
    }

    /// Add a new toast notification.
    ///
    /// Returns the toast ID, or None if the toast was rate-limited or suppressed.
    /// Enforces max visible toasts (T-06-03: unbounded toast spam protection).
    pub fn add(
        &mut self,
        toast_type: ToastType,
        message: String,
        attribution: Option<String>,
        source_panel: Option<PanelId>,
        pattern_id: Option<String>,
        action_text: Option<String>,
        duration: Duration,
    ) -> Option<u64> {
        // Check suppression
        if let (Some(ref pid), Some(ref panel)) = (&pattern_id, &source_panel) {
            if self.is_suppressed(pid, panel) {
                return None;
            }
        }

        // Check rate limiting for pattern+panel combinations
        if let (Some(ref pid), Some(ref panel)) = (&pattern_id, &source_panel) {
            let key = (pid.clone(), *panel);
            if let Some(last) = self.rate_limits.get(&key) {
                if last.elapsed() < RATE_LIMIT_WINDOW {
                    return None;
                }
            }
            self.rate_limits.insert(key, Instant::now());
        }

        let id = self.next_id;
        self.next_id += 1;

        self.toasts.push(Toast {
            id,
            toast_type,
            message,
            attribution,
            source_panel,
            pattern_id,
            action_text,
            shown_at: Instant::now(),
            duration,
        });

        // Enforce max visible toasts by removing oldest if over limit
        while self.toasts.len() > MAX_VISIBLE_TOASTS {
            self.toasts.remove(0);
        }

        Some(id)
    }

    /// Dismiss a specific toast by ID (explicit user action).
    pub fn dismiss(&mut self, toast_id: u64) -> Option<Toast> {
        if let Some(pos) = self.toasts.iter().position(|t| t.id == toast_id) {
            Some(self.toasts.remove(pos))
        } else {
            None
        }
    }

    /// Remove expired toasts. Auto-expiry does NOT call suppress_pattern.
    pub fn tick(&mut self) {
        self.toasts.retain(|t| t.shown_at.elapsed() < t.duration);

        // Clean up old rate limit entries
        self.rate_limits
            .retain(|_, last| last.elapsed() < RATE_LIMIT_WINDOW);
    }

    /// Suppress a pattern for a specific panel (D-07).
    /// Future toasts with this pattern_id+panel_id will be silently dropped.
    pub fn suppress_pattern(&mut self, pattern_id: &str, panel_id: PanelId) {
        self.suppressed
            .entry(pattern_id.to_string())
            .or_default()
            .insert(panel_id);
    }

    /// Check if a pattern is suppressed for a specific panel.
    pub fn is_suppressed(&self, pattern_id: &str, panel_id: &PanelId) -> bool {
        self.suppressed
            .get(pattern_id)
            .map(|panels| panels.contains(panel_id))
            .unwrap_or(false)
    }

    /// Get currently visible toasts (up to MAX_VISIBLE_TOASTS).
    pub fn visible_toasts(&self) -> &[Toast] {
        let len = self.toasts.len();
        let start = if len > MAX_VISIBLE_TOASTS {
            len - MAX_VISIBLE_TOASTS
        } else {
            0
        };
        &self.toasts[start..]
    }

    /// Total number of active toasts.
    pub fn count(&self) -> usize {
        self.toasts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_panel_id(n: u64) -> PanelId {
        PanelId(n)
    }

    #[test]
    fn test_toast_lifecycle() {
        let mut mgr = ToastManager::new();

        // Add a toast with short duration
        let id = mgr
            .add(
                ToastType::Info,
                "Hello".to_string(),
                None,
                None,
                None,
                None,
                Duration::from_millis(50),
            )
            .expect("should add");

        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.visible_toasts()[0].id, id);

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(60));
        mgr.tick();

        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_max_toasts() {
        let mut mgr = ToastManager::new();

        for i in 0..4 {
            mgr.add(
                ToastType::Info,
                format!("Toast {}", i),
                None,
                None,
                None,
                None,
                Duration::from_secs(60),
            );
        }

        // Should enforce max 3
        assert_eq!(mgr.count(), MAX_VISIBLE_TOASTS);
        // Oldest (Toast 0) should have been removed
        assert_eq!(mgr.visible_toasts()[0].message, "Toast 1");
    }

    #[test]
    fn test_suppression() {
        let mut mgr = ToastManager::new();
        let panel = make_panel_id(1);

        assert!(!mgr.is_suppressed("test_pattern", &panel));

        mgr.suppress_pattern("test_pattern", panel);
        assert!(mgr.is_suppressed("test_pattern", &panel));

        // Different panel is not suppressed
        assert!(!mgr.is_suppressed("test_pattern", &make_panel_id(2)));
    }

    #[test]
    fn test_rate_limiting() {
        let mut mgr = ToastManager::new();
        let panel = make_panel_id(1);

        // First toast should succeed
        let result = mgr.add(
            ToastType::Intervention,
            "First".to_string(),
            None,
            Some(panel),
            Some("claude_code".to_string()),
            None,
            Duration::from_secs(60),
        );
        assert!(result.is_some());

        // Second toast with same pattern+panel within 10s should be rejected
        let result = mgr.add(
            ToastType::Intervention,
            "Second".to_string(),
            None,
            Some(panel),
            Some("claude_code".to_string()),
            None,
            Duration::from_secs(60),
        );
        assert!(result.is_none());

        // Toast without pattern_id should always succeed (non-intervention)
        let result = mgr.add(
            ToastType::Conflict,
            "Conflict".to_string(),
            None,
            Some(panel),
            None,
            Some("Undo".to_string()),
            Duration::from_secs(3),
        );
        assert!(result.is_some());
    }

    #[test]
    fn test_dismiss() {
        let mut mgr = ToastManager::new();

        let id = mgr
            .add(
                ToastType::Info,
                "Dismissable".to_string(),
                None,
                None,
                None,
                None,
                Duration::from_secs(60),
            )
            .unwrap();

        assert_eq!(mgr.count(), 1);
        let dismissed = mgr.dismiss(id);
        assert!(dismissed.is_some());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_suppressed_toast_rejected() {
        let mut mgr = ToastManager::new();
        let panel = make_panel_id(1);

        mgr.suppress_pattern("blocked", panel);

        let result = mgr.add(
            ToastType::Intervention,
            "Blocked".to_string(),
            None,
            Some(panel),
            Some("blocked".to_string()),
            None,
            Duration::from_secs(60),
        );
        assert!(result.is_none());
    }
}
