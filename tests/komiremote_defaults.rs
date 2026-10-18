// Dependency-free behavioral tests for the exact module used by the Windows client.
#[path = "../src/komi_config.rs"]
mod komi_config;

use std::collections::HashMap;
use std::sync::{Once, RwLock};

#[test]
fn preserves_existing_default_entries() {
    let once = Once::new();
    let name = RwLock::new("RustDesk".to_owned());
    let server = RwLock::new(HashMap::from([(
        "relay-server".to_owned(),
        "example.org:21117".to_owned(),
    )]));
    let local = RwLock::new(HashMap::from([("lang".to_owned(), "de".to_owned())]));
    komi_config::initialize(&once, &name, &server, &local);
    assert_eq!(
        server.read().unwrap().get("relay-server").unwrap(),
        "example.org:21117"
    );
    assert_eq!(local.read().unwrap().get("lang").unwrap(), "de");
}

#[test]
fn repeated_and_concurrent_calls_do_not_reset_later_choices() {
    let once = Once::new();
    let name = RwLock::new("RustDesk".to_owned());
    let server = RwLock::new(HashMap::new());
    let local = RwLock::new(HashMap::new());
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| komi_config::initialize(&once, &name, &server, &local));
        }
    });
    *name.write().unwrap() = "SignedClient".to_owned();
    server
        .write()
        .unwrap()
        .insert("key".to_owned(), "later-public-key".to_owned());
    local
        .write()
        .unwrap()
        .insert("theme".to_owned(), "light".to_owned());
    local
        .write()
        .unwrap()
        .insert("lang".to_owned(), "".to_owned());
    komi_config::initialize(&once, &name, &server, &local);
    assert_eq!(*name.read().unwrap(), "SignedClient");
    assert_eq!(
        server.read().unwrap().get("key").unwrap(),
        "later-public-key"
    );
    assert_eq!(local.read().unwrap().get("theme").unwrap(), "light");
    assert_eq!(local.read().unwrap().get("lang").unwrap(), "");
}

#[test]
fn initializes_public_defaults_without_writing_user_configuration() {
    let once = Once::new();
    let name = RwLock::new("RustDesk".to_owned());
    let server = RwLock::new(HashMap::new());
    let local = RwLock::new(HashMap::new());
    komi_config::initialize(&once, &name, &server, &local);
    assert_eq!(*name.read().unwrap(), "KomiRemote");
    assert_eq!(
        server
            .read()
            .unwrap()
            .get("custom-rendezvous-server")
            .unwrap(),
        "217.154.20.200"
    );
    assert_eq!(
        server.read().unwrap().get("relay-server").unwrap(),
        "217.154.20.200:21117"
    );
    assert_eq!(
        server.read().unwrap().get("key").unwrap(),
        "52TLfnxb60d5V0VSRwZpUDcDlphr7+qFQqgFumhKpr4="
    );
    assert_eq!(local.read().unwrap().get("lang").unwrap(), "fr");
    assert_eq!(local.read().unwrap().get("theme").unwrap(), "dark");
    assert_eq!(server.read().unwrap().len(), 3);
    assert_eq!(local.read().unwrap().len(), 2);
}
