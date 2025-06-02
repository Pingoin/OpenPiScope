use bno055::{BNO055Calibration, BNO055_CALIB_SIZE};


// Hex-String in Bytes umwandeln
pub fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

// Bytes in Hex-String umwandeln
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn vec_to_calib(vec: Vec<u8>) ->BNO055Calibration {
    let mut array = [0u8; BNO055_CALIB_SIZE];  // Mit Nullen initialisieren
    let len = vec.len().min(BNO055_CALIB_SIZE);
    array[..len].copy_from_slice(&vec[..len]);  // Vorhandene Werte kopieren
    BNO055Calibration::from_buf(&array)
} 