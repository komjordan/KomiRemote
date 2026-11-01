/// Legacy option retained for data compatibility, not a verified public name.
pub const OPTION_KOMI_DEVICE_NAME: &str = "komi-device-name";
pub const MAX_KOMI_DEVICE_NAME_CHARS: usize = 64;

/// Keep user-visible Unicode (including accents), normalize whitespace,
/// remove control characters, and enforce a protocol/UI-safe length.
pub fn sanitize_komi_device_name(value: &str) -> String {
    let without_controls: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    without_controls
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_KOMI_DEVICE_NAME_CHARS)
        .collect()
}

/// Future boundary: only a machine/infrastructure-bound reservation verifier may
/// construct this type. No constructor is exposed while that service is absent.
pub struct VerifiedConnectionName(String);

/// Pure display-name selection; this is never an identity/authentication proof.
pub fn select_device_name(
    verified: Option<&VerifiedConnectionName>,
    system_hostname: &str,
) -> String {
    if let Some(verified) = verified {
        let name = sanitize_komi_device_name(&verified.0);
        if !name.is_empty() {
            return name;
        }
    }
    let hostname = sanitize_komi_device_name(system_hostname);
    if hostname.is_empty() {
        "KomiRemote".to_owned()
    } else {
        hostname
    }
}

/// No verified-name service exists yet. Never read or overwrite the legacy alias.
pub fn advertised_device_name(system_hostname: &str) -> String {
    select_device_name(None, system_hostname)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_accents_and_normalizes_spaces() {
        assert_eq!(
            sanitize_komi_device_name("  Équipe   d’Axel  "),
            "Équipe d’Axel"
        );
    }

    #[test]
    fn removes_controls_and_limits_unicode_characters() {
        assert_eq!(
            sanitize_komi_device_name("Komi\nRemote\tPC\u{7}"),
            "Komi Remote PC"
        );
        let value = "é".repeat(80);
        assert_eq!(sanitize_komi_device_name(&value).chars().count(), 64);
    }

    #[test]
    fn hostname_and_neutral_fallbacks() {
        assert_eq!(advertised_device_name("  windows-pc\n"), "windows-pc");
        assert_eq!(advertised_device_name(" \n\t"), "KomiRemote");
        assert_eq!(advertised_device_name(""), "KomiRemote");
        assert_eq!(advertised_device_name(&"é".repeat(80)).chars().count(), 64);
    }

    #[test]
    fn future_verified_name_has_priority_but_blank_is_not_advertised() {
        let verified = VerifiedConnectionName(" Komjordan ".to_owned());
        assert_eq!(
            select_device_name(Some(&verified), "windows-pc"),
            "Komjordan"
        );
        let blank = VerifiedConnectionName("\n".to_owned());
        assert_eq!(select_device_name(Some(&blank), "windows-pc"), "windows-pc");
    }
}
