//! Public Windows fork defaults. No disk, network, credentials or overwrite settings.
use std::collections::HashMap;
use std::sync::{Once, RwLock};

pub(crate) fn initialize(
    once: &Once,
    app_name: &RwLock<String>,
    server_defaults: &RwLock<HashMap<String, String>>,
    local_defaults: &RwLock<HashMap<String, String>>,
) {
    once.call_once(|| {
        *app_name.write().unwrap() = "KomiRemote".to_owned();
        let mut server = server_defaults.write().unwrap();
        for (key, value) in [
            ("custom-rendezvous-server", "217.154.20.200"),
            ("relay-server", "217.154.20.200:21117"),
            ("key", "52TLfnxb60d5V0VSRwZpUDcDlphr7+qFQqgFumhKpr4="),
            // Engine-enforced manual confirmation by default (connection.rs ApproveMode::Click):
            // passwords are never accepted in this mode. Users can still change it in Settings.
            ("approve-mode", "click"),
        ] {
            server
                .entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
        drop(server);
        let mut local = local_defaults.write().unwrap();
        for (key, value) in [("lang", "fr"), ("theme", "dark")] {
            local
                .entry(key.to_owned())
                .or_insert_with(|| value.to_owned());
        }
    });
}
