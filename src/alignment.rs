use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[atomic_struct::atomic_struct]
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct AlignmentData {
    pub alignment: Option<UnitQuaternion<f32>>,
    pub correction: Option<UnitQuaternion<f32>>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, ToSchema)]
pub struct EulerAngle {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq,Default, Debug,ToSchema)]
pub struct Orientation {
    pub euler:EulerAngle,
    #[schema(value_type = String )]
    pub quaternion: UnitQuaternion<f32>,
}

