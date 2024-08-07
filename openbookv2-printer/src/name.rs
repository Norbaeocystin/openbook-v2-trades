use std::str::from_utf8;

pub fn parse_name(name: &[u8; 16]) -> String {
    let result_name = from_utf8(name);
    // utf8
    if let Ok(name) = result_name {
        return name.to_string();
    }
    // utf16
    let u16_bytes: Vec<u16> = name
        .chunks_exact(2)
        .into_iter()
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();
    let result_name = String::from_utf16(&u16_bytes);
    if let Ok(name) = result_name {
        return name;
    }
    // utf8 with errors
    return String::from_utf8_lossy(name).parse().unwrap();
}
