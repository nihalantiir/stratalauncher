/// Same derivation vanilla uses for offline-mode UUIDs
/// (`UUID.nameUUIDFromBytes` over "OfflinePlayer:<name>").
pub fn offline_uuid(username: &str) -> String {
    let digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes: [u8; 16] = digest.into();

    bytes[6] = (bytes[6] & 0x0f) | 0x30; // version 3
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // RFC 4122 variant

    let hex = hex::encode(bytes);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn is_valid_username(username: &str) -> bool {
    let len_ok = (3..=16).contains(&username.len());
    len_ok
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}
