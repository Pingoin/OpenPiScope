use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[atomic_struct::atomic_struct]
#[derive(Deserialize, Debug, Default, Serialize, Clone, utoipa::ToSchema)]
pub struct GnssData {
    #[schema(value_type = f64)]
    pub lat: f64,
    #[schema(value_type = f64)]
    pub lon: f64,
    #[schema(value_type = f32)]
    pub alt: f32,
    #[schema(value_type = i32)]
    pub leap_seconds: i32,
    #[schema(value_type = f32)]
    pub estimated_error_longitude: f32,
    #[schema(value_type = f32)]
    pub estimated_error_latitude: f32,
    #[schema(value_type = f32)]
    pub estimated_error_plane: f32,
    #[schema(value_type = f32)]
    pub estimated_error_altitude: f32,
    #[schema(value_type = f32)]
    pub track: f32,
    #[schema(value_type = f32)]
    pub speed: f32,
    #[schema(value_type = f32)]
    pub climb: f32,
    #[schema(value_type = Mode)]
    pub mode: Mode,
    #[schema(value_type = f32)]
    pub estimated_error_track: f32,
    #[schema(value_type = f32)]
    pub estimated_error_speed: f32,
    #[schema(value_type = f32)]
    pub estimated_error_climb: f32,
    #[schema(value_type = Vec<Satellite>)]
    pub satellites: Vec<Satellite>,
}

#[derive(
    serde::Deserialize, Default, serde::Serialize, Clone, Copy, Debug, PartialEq, ToSchema,
)]
pub struct Satellite {
    pub prn: i32,
    pub elevation: f32,
    pub azimuth: f32,
    pub signal_strength: f32,
    pub used: bool,
    pub system: GnssSystem,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
}

#[derive(
    Clone,
    Default,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[repr(u8)]
pub enum Mode {
    #[default]
    NoFix = 0,
    Fix2d = 1,
    Fix3d = 2,
}
impl Mode {
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::NoFix => "NO_FIX",
            Self::Fix2d => "FIX_2D",
            Self::Fix3d => "FIX_3D",
        }
    }
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "NO_FIX" => Some(Self::NoFix),
            "FIX_2D" => Some(Self::Fix2d),
            "FIX_3D" => Some(Self::Fix3d),
            _ => None,
        }
    }

    pub fn from_u8(value: u8) -> ::core::option::Option<Self> {
        match value {
            0 => Some(Self::NoFix),
            1 => Some(Self::Fix2d),
            2 => Some(Self::Fix3d),
            _ => None,
        }
    }
}

impl From<gpsd_proto::Mode> for Mode {
    fn from(value: gpsd_proto::Mode) -> Self {
        match value {
            gpsd_proto::Mode::NoFix => Mode::NoFix,
            gpsd_proto::Mode::Fix2d => Mode::Fix2d,
            gpsd_proto::Mode::Fix3d => Mode::Fix3d,
        }
    }
}

#[derive(
    Clone, Copy, Default, Debug, ToSchema,PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum GnssSystem {
    #[default]
    Gps = 0,
    Sbas = 1,
    Galileo = 2,
    Beidou = 3,
    Imes = 4,
    Qzss = 5,
    Glonass = 6,
    Irnss = 7,
}
impl GnssSystem {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Self::Gps => "GPS",
            Self::Sbas => "SBAS",
            Self::Galileo => "GALILEO",
            Self::Beidou => "BEIDOU",
            Self::Imes => "IMES",
            Self::Qzss => "QZSS",
            Self::Glonass => "GLONASS",
            Self::Irnss => "IRNSS",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "GPS" => Some(Self::Gps),
            "SBAS" => Some(Self::Sbas),
            "GALILEO" => Some(Self::Galileo),
            "BEIDOU" => Some(Self::Beidou),
            "IMES" => Some(Self::Imes),
            "QZSS" => Some(Self::Qzss),
            "GLONASS" => Some(Self::Glonass),
            "IRNSS" => Some(Self::Irnss),
            _ => None,
        }
    }
}

impl From<u8> for GnssSystem {
    fn from(value: u8) -> Self {
        match value {
            0 => GnssSystem::Gps,
            1 => GnssSystem::Sbas,
            2 => GnssSystem::Galileo,
            3 => GnssSystem::Beidou,
            4 => GnssSystem::Imes,
            5 => GnssSystem::Qzss,
            6 => GnssSystem::Glonass,
            7 => GnssSystem::Irnss,
            _ => GnssSystem::Gps, // Default case
        }
    }
}

impl From<gpsd_proto::Satellite> for Satellite {
    fn from(value: gpsd_proto::Satellite) -> Self {
        Satellite {
            prn: value.prn as i32,
            elevation: value.el.unwrap_or_default(),
            azimuth: value.az.unwrap_or_default(),
            signal_strength: value.ss.unwrap_or_default(),
            used: value.used,
            system: value.gnssid.unwrap_or_default().into(),
        }
    }
}
