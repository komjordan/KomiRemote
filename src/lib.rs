#[cfg(any(test, not(target_os = "linux")))]
mod audio_resampler;
mod keyboard;
/// cbindgen:ignore
pub mod platform;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use platform::{
    clip_cursor, get_cursor, get_cursor_data, get_cursor_pos, get_focused_display,
    set_cursor_pos, start_os_service,
};
#[cfg(not(any(target_os = "ios")))]
/// cbindgen:ignore
mod server;
#[cfg(not(any(target_os = "ios")))]
pub use self::server::*;
mod client;
mod lan;
#[cfg(not(any(target_os = "ios")))]
mod rendezvous_mediator;
#[cfg(not(any(target_os = "ios")))]
pub use self::rendezvous_mediator::*;
/// cbindgen:ignore
pub mod common;
#[cfg(not(any(target_os = "ios")))]
pub mod ipc;
#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    feature = "flutter"
)))]
pub mod ui;
mod version;
pub use version::*;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
mod bridge_generated;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter;
#[cfg(any(target_os = "android", target_os = "ios", feature = "flutter"))]
pub mod flutter_ffi;
use common::*;
mod auth_2fa;
#[cfg(not(target_os = "ios"))]
mod clipboard;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod core_main;
mod custom_server;
#[cfg(any(windows, target_os = "macos", target_os = "ios"))]
mod komi_config;

/// Read-only iOS name-dialing context. Uses the engine rendezvous server and
/// peer files that actually exist. No signing key, no config write, no host role.
#[cfg(target_os = "ios")]
#[no_mangle]
pub extern "C" fn komi_ios_directory_context() -> *mut std::os::raw::c_char {
    use hbb_common::config::{Config, PeerConfig};
    let result = std::panic::catch_unwind(|| {
        common::initialize_komi_defaults();
        let peers: Vec<String> = PeerConfig::get_vec_id_modified_time_path(&None)
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();
        serde_json::json!({
            "active_server": Config::get_rendezvous_server(),
            "stored_peers": peers,
        })
        .to_string()
    });
    result
        .ok()
        .and_then(|value| std::ffi::CString::new(value).ok())
        .map_or(std::ptr::null_mut(), |value| value.into_raw())
}

/// Free only the pointer returned by komi_ios_directory_context, once.
#[cfg(target_os = "ios")]
#[no_mangle]
pub unsafe extern "C" fn komi_ios_directory_context_free(value: *mut std::os::raw::c_char) {
    if !value.is_null() {
        drop(std::ffi::CString::from_raw(value));
    }
}
pub mod komi_device_name;
pub mod komi_cursor_tag;
mod komi_video_call;
mod lang;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod port_forward;
mod port_forward_mux;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod tray;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod whiteboard;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod updater;

mod ui_cm_interface;
mod ui_interface;
mod ui_session_interface;

mod hbbs_http;

#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub mod clipboard_file;

pub mod privacy_mode;

#[cfg(windows)]
pub mod virtual_display_manager;

mod kcp_stream;
