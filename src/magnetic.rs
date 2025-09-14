#[atomic_struct::atomic_struct]
#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug, utoipa::ToSchema)]
pub struct MagneticData {
    /// in degrees
    #[schema(value_type = f32)]
    pub declination: f32,
    /// in degrees
    #[schema(value_type = f32)]
    pub inclination: f32,
    /// in µT
    #[schema(value_type = f32)]
    pub magnetic_flux_density: f32,
}