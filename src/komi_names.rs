//! Windows/macOS public-name management. Included by flutter_ffi so the exports
//! are linked into librustdesk.dll / liblibrustdesk.dylib without changing the
//! generated Flutter bridge. Windows keeps the seed with DPAPI, macOS in the
//! user's login Keychain; everything else is shared.
#![cfg(any(target_os = "windows", target_os = "macos"))]

use hbb_common::sodiumoxide::{base64, crypto::sign::ed25519 as sign, utils};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf, ptr};
use std::{ffi::CString, io::Read, os::raw::c_char, sync::Mutex, time::Duration};
#[cfg(target_os = "windows")]
use winapi::{shared::minwindef::HLOCAL, um::{dpapi::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN}, winbase::LocalFree, wincrypt::DATA_BLOB}};

static LOCK: Mutex<()> = Mutex::new(());
const MAX_INPUT: usize = 4096;
const MAX_RESPONSE: u64 = 4096;
#[cfg(target_os = "windows")]
const ENTROPY_PREFIX: &[u8] = b"KomiRemote/public-names/v1\0";

fn error(message: &'static str) -> Value { json!({"error": message}) }
fn bad() -> &'static str { "Invalid management request" }
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    v.get(key).and_then(Value::as_str).ok_or_else(bad)
}
fn alias(s: &str) -> bool {
    let b = s.as_bytes();
    (3..=16).contains(&b.len()) && b[0].is_ascii_lowercase() && b[1..].iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-')
        && !matches!(s, "root" | "admin" | "administrator" | "system" | "support" | "komiremote")
}
fn identifier(s: &str) -> bool {
    let b = s.as_bytes();
    (!b.is_empty() && b.len() <= 32 && b.iter().all(u8::is_ascii_digit)) ||
        ((6..=32).contains(&b.len()) && b[0].is_ascii_alphabetic() && b[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-'))
}
fn namespace(s: &str) -> bool {
    !s.is_empty() && s.len() <= 253 && !s.contains("..") && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b':')
        && s.split(':').count() <= 2 && s.split_once(':').map_or(true, |(_, port)| !port.is_empty() && port.len() <= 5 && port.bytes().all(|c| c.is_ascii_digit()))
}
fn endpoint(s: &str) -> Result<url::Url, &'static str> {
    let u = url::Url::parse(s).map_err(|_| bad())?;
    if s.len() > 512 || u.scheme() != "https" || u.host_str().is_none() || !u.username().is_empty() || u.password().is_some() || u.query().is_some() || u.fragment().is_some() || u.path() != "/" { return Err(bad()); }
    Ok(u)
}
fn check_keys(v: &Value, expected: &[&str]) -> Result<(), &'static str> {
    let obj = v.as_object().ok_or_else(bad)?;
    if obj.len() != expected.len() || !expected.iter().all(|k| obj.contains_key(*k)) { return Err(bad()); }
    Ok(())
}
#[cfg(target_os = "windows")]
fn state_path(ns: &str) -> Result<PathBuf, &'static str> {
    let dir = PathBuf::from(std::env::var_os("APPDATA").ok_or("User profile unavailable")?).join("KomiRemote").join("public-names");
    fs::create_dir_all(&dir).map_err(|_| "Protected storage unavailable")?;
    let hash = Sha256::digest(ns.as_bytes());
    Ok(dir.join(format!("{:x}.dpapi", hash)))
}
#[cfg(target_os = "windows")]
fn entropy(ns: &str) -> Vec<u8> { let mut x = ENTROPY_PREFIX.to_vec(); x.extend_from_slice(ns.as_bytes()); x }
#[cfg(target_os = "windows")]
fn blob(data: &mut [u8]) -> DATA_BLOB { DATA_BLOB { cbData: data.len() as u32, pbData: data.as_mut_ptr() } }
#[cfg(target_os = "windows")]
fn protect(plain: &mut [u8], ns: &str) -> Result<Vec<u8>, &'static str> {
    let mut entropy = entropy(ns);
    let mut output: DATA_BLOB = unsafe { std::mem::zeroed() };
    let ok = unsafe { CryptProtectData(&mut blob(plain), ptr::null(), &mut blob(&mut entropy), ptr::null_mut(), ptr::null_mut(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) };
    if ok == 0 { return Err("DPAPI protection failed"); }
    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData as HLOCAL); }
    Ok(result)
}
#[cfg(target_os = "windows")]
fn unprotect(data: &mut [u8], ns: &str) -> Result<Vec<u8>, &'static str> {
    let mut entropy = entropy(ns);
    let mut output: DATA_BLOB = unsafe { std::mem::zeroed() };
    let ok = unsafe { CryptUnprotectData(&mut blob(data), ptr::null_mut(), &mut blob(&mut entropy), ptr::null_mut(), ptr::null_mut(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) };
    if ok == 0 { return Err("Protected identity cannot be opened"); }
    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { ptr::write_bytes(output.pbData, 0, output.cbData as usize); LocalFree(output.pbData as HLOCAL); }
    Ok(result)
}

// Only the 32-byte Ed25519 seed is stored; never serialize the full secret key.
#[cfg(target_os = "windows")]
fn load(ns: &str) -> Result<Option<sign::Seed>, &'static str> {
    let path = state_path(ns)?;
    let mut f = match fs::File::open(path) { Ok(f) => f, Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None), Err(_) => return Err("Protected storage unavailable") };
    let mut encrypted = Vec::new();
    f.take(2049).read_to_end(&mut encrypted).map_err(|_| "Protected storage unavailable")?;
    if encrypted.is_empty() || encrypted.len() > 2048 { return Err("Protected identity is invalid"); }
    let mut plain = unprotect(&mut encrypted, ns)?;
    if plain.len() != 32 { utils::memzero(&mut plain); return Err("Protected identity is invalid"); }
    let mut seed = [0u8; 32]; seed.copy_from_slice(&plain); utils::memzero(&mut plain);
    Ok(Some(sign::Seed(seed)))
}
#[cfg(target_os = "windows")]
fn save_new(ns: &str, seed: &mut [u8; 32]) -> Result<(), &'static str> {
    let mut encrypted = protect(seed, ns)?;
    let path = state_path(ns)?;
    let result = (|| {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|_| "Protected identity already exists")?;
        if f.write_all(&encrypted).and_then(|_| f.sync_all()).is_err() {
            // Do not remove an ambiguous partially written key: fail closed.
            return Err("Protected storage unavailable");
        }
        Ok(())
    })();
    utils::memzero(&mut encrypted);
    result?;
    let recovered = load(ns)?.ok_or("Protected identity cannot be opened")?;
    if recovered.0 != *seed { return Err("Protected identity cannot be opened"); }
    Ok(())
}
// macOS: the seed is a generic password in the user's login Keychain
// (service "KomiRemote public-names v1", account = SHA-256 of the namespace).
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "KomiRemote public-names v1";
#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
#[cfg(target_os = "macos")]
fn keychain_account(ns: &str) -> String { format!("{:x}", Sha256::digest(ns.as_bytes())) }
#[cfg(target_os = "macos")]
fn load(ns: &str) -> Result<Option<sign::Seed>, &'static str> {
    use security_framework::passwords::get_generic_password;
    let mut plain = match get_generic_password(KEYCHAIN_SERVICE, &keychain_account(ns)) {
        Ok(v) => v,
        Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => return Ok(None),
        Err(_) => return Err("Protected identity cannot be opened"),
    };
    if plain.len() != 32 { utils::memzero(&mut plain); return Err("Protected identity is invalid"); }
    let mut seed = [0u8; 32]; seed.copy_from_slice(&plain); utils::memzero(&mut plain);
    Ok(Some(sign::Seed(seed)))
}
#[cfg(target_os = "macos")]
fn save_new(ns: &str, seed: &mut [u8; 32]) -> Result<(), &'static str> {
    use security_framework::passwords::set_generic_password;
    // Never overwrite an existing identity (same rule as create_new on Windows).
    if load(ns)?.is_some() { return Err("Protected identity already exists"); }
    set_generic_password(KEYCHAIN_SERVICE, &keychain_account(ns), &seed[..]).map_err(|_| "Protected storage unavailable")?;
    let recovered = load(ns)?.ok_or("Protected identity cannot be opened")?;
    if recovered.0 != *seed { return Err("Protected identity cannot be opened"); }
    Ok(())
}
fn client() -> Result<reqwest::blocking::Client, &'static str> {
    reqwest::blocking::Client::builder().redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5)).connect_timeout(Duration::from_secs(5)).build().map_err(|_| "Registry unavailable")
}
fn request(c: &reqwest::blocking::Client, url: &url::Url, path: &str, body: Option<Value>, status: u16) -> Result<Value, &'static str> {
    let u = url.join(path).map_err(|_| bad())?;
    let mut response = if let Some(body) = body { c.post(u).json(&body).send() } else { c.get(u).send() }.map_err(|_| "Registry unavailable")?;
    if response.status().as_u16() != status {
        return Err(match response.status().as_u16() {
            400 => "Nom ou requête invalide",
            401 | 403 => "Identité de réservation refusée",
            404 => "Nom introuvable",
            409 => "Nom déjà pris ou ID déjà réservé",
            429 => "Trop de requêtes ; réessayez plus tard",
            _ => "Registre indisponible",
        });
    }
    if response.content_length().map_or(false, |n| n > MAX_RESPONSE) { return Err("Invalid registry response"); }
    let mut bytes = Vec::new();
    response.take(MAX_RESPONSE + 1).read_to_end(&mut bytes).map_err(|_| "Registry unavailable")?;
    if bytes.len() as u64 > MAX_RESPONSE { return Err("Invalid registry response"); }
    serde_json::from_slice(&bytes).map_err(|_| "Invalid registry response")
}
fn public(seed: &sign::Seed) -> (String, sign::SecretKey) {
    let (pk, sk) = sign::keypair_from_seed(seed);
    (base64::encode(pk.0, base64::Variant::Original), sk)
}
fn verify_record(v: &Value, a: &str, id: &str, ns: &str, pk: &str) -> Result<(), &'static str> {
    if v.get("alias").and_then(Value::as_str) != Some(a) || v.get("rustdesk_id").and_then(Value::as_str) != Some(id)
        || v.get("server").and_then(Value::as_str) != Some(ns) || v.get("public_key").and_then(Value::as_str) != Some(pk) { return Err("Untrusted registry response"); }
    Ok(())
}
fn resolve(c: &reqwest::blocking::Client, url: &url::Url, a: &str, id: &str, ns: &str, pk: &str) -> Result<Value, &'static str> {
    let v = request(c, url, &format!("resolve/{a}"), None, 200)?;
    verify_record(&v, a, id, ns, pk)?; Ok(v)
}
fn challenge(c: &reqwest::blocking::Client, url: &url::Url, op: &str, old: &str, new: Option<&str>, id: &str, ns: &str) -> Result<Value, &'static str> {
    let v = request(c, url, "challenge", Some(json!({"operation":op,"old_alias":old,"new_alias":new,"rustdesk_id":id})), 200)?;
    check_keys(&v, &["nonce", "operation", "old_alias", "new_alias", "rustdesk_id", "namespace", "expires_at"])?;
    let nonce = field(&v, "nonce")?;
    if nonce.is_empty() || nonce.len() > 128 || !nonce.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || field(&v, "operation")? != op || field(&v, "old_alias")? != old || v.get("new_alias") != Some(&new.map_or(Value::Null, |n| json!(n)))
        || field(&v, "rustdesk_id")? != id || field(&v, "namespace")? != ns { return Err("Untrusted registry challenge"); }
    let expiry = v.get("expires_at").and_then(Value::as_i64).ok_or("Untrusted registry challenge")?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| "Clock unavailable")?.as_secs() as i64;
    if expiry < now || expiry > now + 120 { return Err("Untrusted registry challenge"); }
    Ok(v)
}
fn run(v: Value) -> Result<Value, &'static str> {
    let op = field(&v, "operation")?;
    let keys: &[&str] = match op {
        "active-server" => &["operation", "namespace"],
        "status" => &["operation", "namespace", "id", "url"],
        "availability" => &["operation", "alias", "namespace", "url"],
        "register" => &["operation", "alias", "id", "namespace", "url"],
        "rename" => &["operation", "alias", "new_alias", "id", "namespace", "url"],
        "release" => &["operation", "alias", "id", "namespace", "url"],
        _ => return Err(bad()),
    };
    check_keys(&v, keys)?;
    let ns = field(&v, "namespace")?;
    if !namespace(ns) { return Err(bad()); }
    let active = hbb_common::config::Config::get_rendezvous_server();
    if active != ns && active != format!("{ns}:21116") {
        return Err("Le serveur RustDesk actif ne correspond pas au registre de noms");
    }
    if op == "active-server" { return Ok(json!({"matched":true, "namespace":ns})); }
    if op == "status" {
        hbb_common::sodiumoxide::init().map_err(|_| "Cryptography unavailable")?;
        let id = field(&v, "id")?;
        if !identifier(id) { return Err(bad()); }
        let u = endpoint(field(&v, "url")?)?;
        if let Some(seed) = load(ns)? {
            let (pk, _) = public(&seed);
            let raw = base64::decode(&pk, base64::Variant::Original).map_err(|_| "Cryptography unavailable")?;
            let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
            let c = client()?;
            let mut response = c.get(u.join(&format!("by-key/{hex}")).map_err(|_| bad())?).send().map_err(|_| "Registry unavailable")?;
            if response.status().as_u16() == 404 {
                return Ok(json!({"has_identity":true, "namespace":ns, "alias":null}));
            }
            if response.status().as_u16() != 200 { return Err("Registry unavailable"); }
            let mut bytes = Vec::new();
            response.take(MAX_RESPONSE + 1).read_to_end(&mut bytes).map_err(|_| "Registry unavailable")?;
            if bytes.len() > MAX_RESPONSE as usize { return Err("Untrusted registry response"); }
            let record: Value = serde_json::from_slice(&bytes).map_err(|_| "Untrusted registry response")?;
            let found = field(&record, "alias")?;
            if !alias(found) { return Err("Untrusted registry response"); }
            verify_record(&record, found, id, ns, &pk)?;
            return Ok(json!({"has_identity":true, "namespace":ns, "alias":found}));
        }
        return Ok(json!({"has_identity":false, "namespace":ns, "alias":null}));
    }
    let a = field(&v, "alias")?;
    if !alias(a) { return Err(bad()); }
    let u = endpoint(field(&v, "url")?)?;
    let c = client()?;
    if op == "availability" {
        let result = request(&c, &u, &format!("availability/{a}"), None, 200)?;
        if result.get("alias").and_then(Value::as_str) != Some(a) || result.get("server").and_then(Value::as_str) != Some(ns) || result.get("available").and_then(Value::as_bool).is_none() { return Err("Untrusted registry response"); }
        return Ok(result);
    }
    let id = field(&v, "id")?;
    if !identifier(id) { return Err(bad()); }
    // No destructive operation or registration may silently reset a damaged identity.
    hbb_common::sodiumoxide::init().map_err(|_| "Cryptography unavailable")?;
    let seed = match load(ns)? {
        Some(seed) => seed,
        None if op == "register" => {
            let (_, mut sk) = sign::gen_keypair();
            let mut raw = [0u8;32]; raw.copy_from_slice(&sk.0[..32]);
            utils::memzero(&mut sk.0);
            let result = save_new(ns, &mut raw);
            let seed = sign::Seed(raw);
            utils::memzero(&mut raw);
            if let Err(e) = result { return Err(e); }
            seed
        }
        None => return Err("No management identity for namespace"),
    };
    let (pk, sk) = public(&seed);
    if op == "register" {
        let result = request(&c, &u, "register", Some(json!({"alias":a,"rustdesk_id":id,"public_key":pk})), 201)?;
        verify_record(&result, a, id, ns, &pk)?;
        return Ok(result);
    }
    resolve(&c, &u, a, id, ns, &pk)?;
    let next = if op == "rename" { Some(field(&v, "new_alias")?) } else { None };
    if let Some(next) = next { if !alias(next) || next == a { return Err(bad()); } }
    let challenge = challenge(&c, &u, op, a, next, id, ns)?;
    // serde_json::Map's default representation is BTreeMap (sorted keys).
    // Explicitly canonicalize the seven fields so preserve_order builds also sign correctly.
    let ordered: std::collections::BTreeMap<&str, &Value> = challenge.as_object().ok_or("Untrusted registry challenge")?.iter().map(|(k,v)| (k.as_str(),v)).collect();
    let mut message = b"komi-names-v1\n".to_vec();
    message.extend(serde_json::to_vec(&ordered).map_err(|_| "Untrusted registry challenge")?);
    let sig = sign::sign_detached(&message, &sk);
    let signature = base64::encode(sig.to_bytes(), base64::Variant::Original);
    let result = request(&c, &u, op, Some(json!({"challenge":challenge,"signature":signature})), 200)?;
    if let Some(next) = next {
        verify_record(&result, next, id, ns, &pk)?;
        resolve(&c, &u, next, id, ns, &pk)?;
    } else {
        if result.get("released").and_then(Value::as_bool) != Some(true) { return Err("Untrusted registry response"); }
        // Retain the DPAPI/Keychain identity: uncertain network completion must never
        // silently rotate the key. A released key can be used for a new claim.
    }
    Ok(result)
}

/// Input: NUL-terminated UTF-8 JSON <=4096 bytes. Output: owned UTF-8 JSON
/// allocated by Rust; ALWAYS release with komi_names_free, including errors.
/// No key material is ever returned. Calls are serialized within this process.
#[no_mangle]
pub unsafe extern "C" fn komi_names_call(json_utf8: *const c_char) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        if json_utf8.is_null() { return error(bad()); }
        let mut length = 0;
        while length <= MAX_INPUT {
            if unsafe { *json_utf8.add(length) } == 0 { break; }
            length += 1;
        }
        if length > MAX_INPUT { return error(bad()); }
        let bytes = unsafe { std::slice::from_raw_parts(json_utf8 as *const u8, length) };
        let input: Value = match serde_json::from_slice(bytes) { Ok(v) => v, Err(_) => return error(bad()) };
        let _guard = match LOCK.lock() { Ok(g) => g, Err(_) => return error("Management identity unavailable") };
        match run(input) { Ok(v) => v, Err(msg) => error(msg) }
    }).unwrap_or_else(|_| error("Management request failed"));
    CString::new(result.to_string()).unwrap_or_else(|_| CString::new("{\"error\":\"Management request failed\"}").unwrap()).into_raw()
}

#[no_mangle]
pub unsafe extern "C" fn komi_names_free(value: *mut c_char) {
    if !value.is_null() { drop(unsafe { CString::from_raw(value) }); }
}
