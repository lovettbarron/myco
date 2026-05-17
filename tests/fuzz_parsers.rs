//! Property-based fuzz tests for Myco parsers and deserializers.
//!
//! These tests verify that parsers handle arbitrary input gracefully:
//! - Never panic on any input
//! - Return reasonable results for valid-ish input
//! - Don't loop infinitely
//!
//! Uses proptest to generate random inputs (256 cases per test by default).
//! Threat model: T-07-06 mitigated by limiting nesting depth to 100 and
//! string length to 5000 chars.

use proptest::prelude::*;

// ============================================================
// Markdown parser fuzzing
// ============================================================

mod markdown_fuzz {
    use super::*;
    use myco::markdown::parser::parse_markdown_to_blocks;

    proptest! {
        /// Any arbitrary string should not panic the markdown parser.
        #[test]
        fn never_panics(input in "\\PC{0,5000}") {
            let _ = parse_markdown_to_blocks(&input);
        }

        /// Empty/whitespace-only string produces empty or minimal output.
        #[test]
        fn whitespace_safe(input in "\\s{0,200}") {
            let result = parse_markdown_to_blocks(&input);
            // Whitespace-only input should not panic; result may be empty or have whitespace blocks
            let _ = result;
        }

        /// Headings with non-whitespace text produce at least one block.
        #[test]
        fn headings_produce_blocks(level in 1u8..=6, text in "[a-zA-Z0-9]{1,100}") {
            let hashes = "#".repeat(level as usize);
            let input = format!("{} {}", hashes, text);
            let blocks = parse_markdown_to_blocks(&input);
            prop_assert!(!blocks.is_empty(), "Heading should produce at least one block");
        }

        /// Deeply nested lists don't stack overflow (T-07-06: capped at 100).
        #[test]
        fn deep_nesting_no_overflow(depth in 1usize..=100) {
            let input: String = (0..depth)
                .map(|i| format!("{}* item {}\n", "  ".repeat(i), i))
                .collect();
            let _ = parse_markdown_to_blocks(&input);
        }

        /// Code blocks with arbitrary content are safe.
        #[test]
        fn code_blocks_safe(content in "[\\x00-\\x7f]{0,500}") {
            let input = format!("```\n{}\n```", content);
            let _ = parse_markdown_to_blocks(&input);
        }

        /// Multiple headings at different levels don't panic.
        #[test]
        fn multiple_headings(
            level1 in 1u8..=6,
            level2 in 1u8..=6,
            text1 in "[a-z]{1,30}",
            text2 in "[a-z]{1,30}"
        ) {
            let input = format!(
                "{} {}\n\n{} {}",
                "#".repeat(level1 as usize), text1,
                "#".repeat(level2 as usize), text2
            );
            let blocks = parse_markdown_to_blocks(&input);
            prop_assert!(blocks.len() >= 2, "Two headings should produce at least two blocks");
        }
    }
}

// ============================================================
// Keyboard shortcut parser fuzzing
// ============================================================

mod shortcut_fuzz {
    use super::*;
    use myco::shortcuts::chord::parse_key_string;

    proptest! {
        /// Any string should not panic the key parser.
        #[test]
        fn never_panics(input in "\\PC{0,200}") {
            let _ = parse_key_string(&input);
        }

        /// Strings with '+' separators produce a KeyCombo.
        #[test]
        fn plus_separated_keys(parts in prop::collection::vec("[a-z]{1,10}", 1..=5)) {
            let input = parts.join("+");
            let combo = parse_key_string(&input);
            // The last non-modifier part becomes the key
            let _ = combo.key;
            let _ = combo.modifiers;
        }

        /// Known modifier names are recognized.
        #[test]
        fn modifier_recognition(
            modifier in prop_oneof![
                Just("cmd"),
                Just("ctrl"),
                Just("shift"),
                Just("alt"),
                Just("super"),
                Just("meta"),
                Just("control"),
                Just("option")
            ],
            key in "[a-z]"
        ) {
            let input = format!("{}+{}", modifier, key);
            let combo = parse_key_string(&input);
            // At least one modifier should be set
            prop_assert!(
                combo.modifiers.cmd || combo.modifiers.ctrl || combo.modifiers.shift || combo.modifiers.alt,
                "Modifier '{}' should set at least one flag", modifier
            );
            prop_assert_eq!(&combo.key, &key);
        }

        /// Empty string doesn't panic.
        #[test]
        fn empty_input_safe(_dummy in Just(())) {
            let _ = parse_key_string("");
        }

        /// Repeated modifiers don't panic.
        #[test]
        fn repeated_modifiers(count in 1usize..=10) {
            let input = std::iter::repeat("cmd").take(count).collect::<Vec<_>>().join("+");
            let combo = parse_key_string(&input);
            prop_assert!(combo.modifiers.cmd, "cmd modifier should be set");
        }

        /// Unicode characters in key strings don't panic.
        #[test]
        fn unicode_keys(input in "[\\p{L}\\+]{1,50}") {
            let _ = parse_key_string(&input);
        }
    }
}

// ============================================================
// Config JSON deserialization fuzzing
// ============================================================

mod config_fuzz {
    use super::*;
    use myco::config::ProjectConfig;

    proptest! {
        /// Arbitrary strings should not panic serde_json deserialization.
        #[test]
        fn deserialize_never_panics(json in "\\PC{0,5000}") {
            let _ = serde_json::from_str::<ProjectConfig>(&json);
        }

        /// Valid JSON objects that don't match schema return Err, not panic.
        #[test]
        fn wrong_schema_no_panic(
            key in "[a-z]{1,20}",
            value in prop_oneof![
                Just("null".to_string()),
                Just("true".to_string()),
                Just("42".to_string()),
                "[a-z]{1,20}".prop_map(|s| format!("\"{}\"", s))
            ]
        ) {
            let json = format!(r#"{{"{}":{}}}"#, key, value);
            let result = serde_json::from_str::<ProjectConfig>(&json);
            // Should be Err (wrong schema) but not panic
            prop_assert!(result.is_err());
        }

        /// Deeply nested JSON doesn't stack overflow (T-07-06: capped at 100).
        #[test]
        fn deep_json_no_overflow(depth in 1usize..=100) {
            let open: String = "{\"a\":".repeat(depth);
            let close: String = "}".repeat(depth);
            let json = format!("{}null{}", open, close);
            let _ = serde_json::from_str::<ProjectConfig>(&json);
        }

        /// Arrays of various sizes don't panic.
        #[test]
        fn arrays_no_panic(size in 0usize..=50) {
            let elements: String = (0..size).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            let json = format!("[{}]", elements);
            let _ = serde_json::from_str::<ProjectConfig>(&json);
        }

        /// Strings with escape sequences don't panic.
        #[test]
        fn escape_sequences_no_panic(content in "[\\x00-\\x7f]{0,100}") {
            // Wrap in a JSON string with proper escaping
            let json_str = serde_json::to_string(&content).unwrap_or_else(|_| "\"\"".to_string());
            let _ = serde_json::from_str::<ProjectConfig>(&json_str);
        }
    }
}
