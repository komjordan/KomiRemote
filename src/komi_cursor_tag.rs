//! KomiRemote: who is moving the shared remote cursor, and which name a
//! controller presents to the host. Dependency-free so it can be tested alone
//! (see tests/komi_cursor_tag.rs).

/// A controller's name is shown next to the remote cursor this long after its
/// last mouse input.
pub const CONTROLLER_NAME_TTL_MS: i64 = 3000;
/// The controller is "the one moving the cursor" for this long; it does not
/// need its own position echoed back (mirrors input_service's 300 ms rule).
pub const CONTROLLER_ACTIVE_MS: i64 = 300;
pub const MAX_CONTROLLER_NAME_CHARS: usize = 32;
/// Local option holding the name chosen in KomiRemote (its confirmed public
/// connection name), written by the UI; read when logging in to a peer.
pub const OPTION_KOMI_DISPLAY_NAME: &str = "komi-display-name";

/// Remove control characters, collapse whitespace and cap the length.
pub fn sanitize_controller_name(value: &str) -> String {
    let without_controls: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    without_controls
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_CONTROLLER_NAME_CHARS)
        .collect()
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CursorController {
    pub conn: i32,
    pub name: String,
    pub time: i64,
}

impl CursorController {
    /// Record a mouse input from connection `conn`, presented as `name`.
    pub fn note(&mut self, conn: i32, name: &str, now: i64) {
        if self.conn != conn || self.name.is_empty() {
            self.name = sanitize_controller_name(name);
        }
        self.conn = conn;
        self.time = now;
    }

    /// Forget a closed connection so its name never lingers.
    pub fn clear_conn(&mut self, conn: i32) {
        if self.conn == conn {
            *self = Self::default();
        }
    }

    fn is_fresh(&self, now: i64, window: i64) -> bool {
        self.conn != 0 && now >= self.time && now - self.time < window
    }

    /// Name to attach to a cursor position sent to the other viewers, or empty.
    pub fn broadcast_name(&self, now: i64) -> String {
        if self.is_fresh(now, CONTROLLER_NAME_TTL_MS) {
            self.name.clone()
        } else {
            String::new()
        }
    }

    /// Name to attach for the other viewers while that controller is actually moving
    /// the cursor (same window as [`Self::active_conn`]); empty otherwise, so moves made
    /// by the local user never carry a remote name.
    pub fn moving_name(&self, now: i64) -> String {
        if self.is_fresh(now, CONTROLLER_ACTIVE_MS) {
            self.name.clone()
        } else {
            String::new()
        }
    }

    /// The connection currently moving the cursor, if any.
    pub fn active_conn(&self, now: i64) -> Option<i32> {
        if self.is_fresh(now, CONTROLLER_ACTIVE_MS) {
            Some(self.conn)
        } else {
            None
        }
    }
}

/// Name a controller presents: the name chosen in KomiRemote first, then an
/// account/build-provided name, then the OS user name, then the host name.
pub fn select_display_name(
    chosen: &str,
    account: &str,
    os_username: &str,
    hostname: &str,
) -> (String, bool) {
    let chosen = sanitize_controller_name(chosen);
    if !chosen.is_empty() {
        // Shown exactly as the user typed it.
        return (chosen, true);
    }
    for candidate in [account, os_username, hostname] {
        let candidate = candidate.trim();
        if !candidate.is_empty() {
            return (candidate.to_owned(), false);
        }
    }
    (String::new(), false)
}
