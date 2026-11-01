// Dependency-free behavioral tests for the exact module used by the Windows client.
#[path = "../src/komi_cursor_tag.rs"]
mod komi_cursor_tag;

use komi_cursor_tag::*;

#[test]
fn sanitizes_controller_names() {
    assert_eq!(sanitize_controller_name("  Axel\n\t Kom  "), "Axel Kom");
    assert_eq!(sanitize_controller_name("\u{7}"), "");
    assert_eq!(
        sanitize_controller_name(&"é".repeat(80)).chars().count(),
        MAX_CONTROLLER_NAME_CHARS
    );
}

#[test]
fn name_is_broadcast_only_while_recent() {
    let mut c = CursorController::default();
    assert_eq!(c.broadcast_name(1_000), "");
    c.note(7, "kom", 1_000);
    assert_eq!(c.broadcast_name(1_000), "kom");
    assert_eq!(c.broadcast_name(1_000 + CONTROLLER_NAME_TTL_MS - 1), "kom");
    assert_eq!(c.broadcast_name(1_000 + CONTROLLER_NAME_TTL_MS), "");
    // A clock going backwards never shows a stale name.
    assert_eq!(c.broadcast_name(999), "");
}

#[test]
fn active_controller_is_excluded_only_briefly() {
    let mut c = CursorController::default();
    c.note(7, "kom", 1_000);
    assert_eq!(c.active_conn(1_100), Some(7));
    assert_eq!(c.active_conn(1_000 + CONTROLLER_ACTIVE_MS), None);
}

#[test]
fn latest_controller_wins_and_closed_connections_are_forgotten() {
    let mut c = CursorController::default();
    c.note(7, "kom", 1_000);
    c.note(8, "Alice", 1_500);
    assert_eq!(c.broadcast_name(1_600), "Alice");
    c.clear_conn(7);
    assert_eq!(c.broadcast_name(1_600), "Alice");
    c.clear_conn(8);
    assert_eq!(c.broadcast_name(1_600), "");
    assert_eq!(c, CursorController::default());
}

#[test]
fn display_name_prefers_the_chosen_name() {
    assert_eq!(
        select_display_name(" kom ", "", "root", "PC-1"),
        ("kom".to_owned(), true)
    );
    assert_eq!(
        select_display_name("", "", "root", "PC-1"),
        ("root".to_owned(), false)
    );
    assert_eq!(
        select_display_name("", "Account", "root", "PC-1"),
        ("Account".to_owned(), false)
    );
    assert_eq!(
        select_display_name("", "", " ", "PC-1"),
        ("PC-1".to_owned(), false)
    );
    assert_eq!(select_display_name("", "", "", ""), (String::new(), false));
}
