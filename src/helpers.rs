use std::cell::RefCell;
use bno055::BNO055_CALIB_SIZE;
pub struct MutexBox<T>(critical_section::Mutex<RefCell<T>>);

impl<T> MutexBox<T> {
    pub const fn new(value: T) -> MutexBox<T> {
        MutexBox(critical_section::Mutex::new(RefCell::new(value)))
    }

    pub fn open<F, R>(&self, f: F) -> R
    where
        F: Fn(&mut T) -> R,
    {
        critical_section::with(|cs| {
            let mut data = self.0.borrow_ref_mut(cs);
            f(&mut data)
        })
    }
    pub fn clone_inner(&self) -> T
    where
        T: Clone,
    {
        self.open(|c| c.clone())
    }
}


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

pub fn vec_to_calib(vec: Vec<u8>) -> [u8; BNO055_CALIB_SIZE] {
    let mut array = [0u8; BNO055_CALIB_SIZE];  // Mit Nullen initialisieren
    let len = vec.len().min(BNO055_CALIB_SIZE);
    array[..len].copy_from_slice(&vec[..len]);  // Vorhandene Werte kopieren
    array
}