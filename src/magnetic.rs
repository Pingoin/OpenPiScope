#[atomic_struct::atomic_struct]
#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug)]
pub struct MagneticData {
    /// in degrees
    pub declination: f32,
    /// in degrees
    pub inclination: f32,
    /// in µT
    pub magnetic_flux_density: f32,
}