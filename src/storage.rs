use bno055::BNO055_CALIB_SIZE;
use chrono::Datelike;
use gpsd_proto::UnifiedResponse;
use nalgebra::UnitQuaternion;
use std::fs;
use std::path::Path;
use tokio_util::codec::LinesCodecError;
use toml_edit::{value, DocumentMut};
use world_magnetic_model::{
    time::Date,
    uom::si::{
        angle::{degree, Angle},
        f32::Length,
        length::meter,
        magnetic_flux_density::microtesla,
    },
    GeomagneticField,
};

use crate::{
    generated::open_pi_scope::{
        AlignmentData, EulerAngle, GnssData, MagneticData, Position, Quaternion,
    },
    helpers::{hex_decode, hex_encode, vec_to_calib, MutexBox},
};

const CONFIG_PATH: &str = "/boot/open-pi-scope";

pub(crate) struct Storage {
    gnss_data: MutexBox<GnssData>,
    magnetic_data: MutexBox<MagneticData>,
    alingment_data: MutexBox<AlignmentData>,
    config: MutexBox<Option<DocumentMut>>,
}

impl Storage {
    pub const fn new() -> Self {
        Storage {
            gnss_data: MutexBox::new(GnssData {
                lat: 0.0,
                lon: 0.0,
                alt: 0.0,
                leap_seconds: 0,
                estimated_error_longitude: 0.0,
                estimated_error_latitude: 0.0,
                estimated_error_plane: 0.0,
                estimated_error_altitude: 0.0,
                track: 0.0,
                speed: 0.0,
                climb: 0.0,
                mode: 0,
                estimated_error_track: 0.0,
                estimated_error_speed: 0.0,
                estimated_error_climb: 0.0,
                satellites: Vec::new(),
            }),
            magnetic_data: MutexBox::new(MagneticData {
                declination: 0.0,
                inclination: 0.0,
                magnetic_flux_density: 0.0,
            }),
            alingment_data: MutexBox::new(AlignmentData {
                alignment: None,
                correction: None,
            }),
            config: MutexBox::new(None),
        }
    }
    pub fn load_config(&self) -> anyhow::Result<()> {
        // Datei einlesen
        let content = fs::read_to_string(CONFIG_PATH)?;
        let doc = content.parse::<DocumentMut>()?;
        // TOML-Dokument parsen (Kommentare bleiben erhalten)
        self.config
            .open(move |document| *document = Some(doc.clone()));
        Ok(())
    }

    pub async fn update_gpsd(&self, line: String) -> Result<(), LinesCodecError> {
        match serde_json::from_str(&line) {
            Ok(rd) => match rd {
                UnifiedResponse::Tpv(t) => {
                    self.gnss_data.open(|data| {
                        data.lat = t.lat.unwrap_or_default();
                        data.lon = t.lon.unwrap_or_default();
                        data.alt = t.alt.unwrap_or_default();
                        data.leap_seconds = t.leapseconds.unwrap_or_default();
                        data.estimated_error_longitude = t.epx.unwrap_or_default();
                        data.estimated_error_latitude = t.epy.unwrap_or_default();
                        data.estimated_error_plane = t.eph.unwrap_or_default();
                        data.estimated_error_altitude = t.epv.unwrap_or_default();
                        data.track = t.track.unwrap_or_default();
                        data.speed = t.speed.unwrap_or_default();
                        data.mode = t.mode as i32;
                        data.climb = t.climb.unwrap_or_default();
                        data.estimated_error_track = t.epd.unwrap_or_default();
                        data.estimated_error_speed = t.eps.unwrap_or_default();
                        data.estimated_error_climb = t.epc.unwrap_or_default();
                        println!("Fix: {} / Sattelites: {}", t.mode, data.satellites.len());
                    });
                    self.update_magnetic();
                }
                UnifiedResponse::Sky(s) => {
                    self.gnss_data.open(|data| {
                        if let Some(sats) = s.satellites.clone() {
                            data.satellites = sats.iter().map(|sat| sat.clone().into()).collect();
                        }
                    });
                }
                _ => {}
            },
            Err(e) => {
                println!("Error decoding: {e}");
            }
        };
        Ok(())
    }
    fn update_magnetic(&self) {
        let pos = self.get_position();
        self.magnetic_data.open(|mag| {
            let now = chrono::Utc::now();

            if let Ok(geomagnetic_field) = GeomagneticField::new(
                Length::new::<meter>(pos.altitude),         // height
                Angle::new::<degree>(pos.latitude as f32),  // lat
                Angle::new::<degree>(pos.longitude as f32), // lon
                Date::from_ordinal_date(now.year(), now.ordinal() as u16).unwrap_or(Date::MIN), // date
            ) {
                mag.declination = geomagnetic_field.declination().get::<degree>();
                mag.inclination = geomagnetic_field.inclination().get::<degree>();
                mag.magnetic_flux_density = geomagnetic_field.f().get::<microtesla>();
            };
        })
    }
    pub fn get_gnss_data(&self) -> GnssData {
        self.gnss_data.clone_inner()
    }
    pub fn get_magnetic_data(&self) -> MagneticData {
        self.magnetic_data.clone_inner()
    }
    pub fn get_position(&self) -> Position {
        self.gnss_data.open(|gnss| Position {
            latitude: gnss.lat,
            longitude: gnss.lon,
            altitude: gnss.alt,
        })
    }
    pub fn update_orientation(&self, orientation: Quaternion) {
        self.alingment_data.open(|alignment| {
            alignment.alignment = Some(orientation);
        })
    }
    pub fn get_orientation(&self) -> Option<(Quaternion, EulerAngle)> {
        let alignment = self.alingment_data.clone_inner();
        let quat: UnitQuaternion<f32> = alignment.alignment?.into();
        let quat = if let Some(correction) = alignment.correction {
            let correction: UnitQuaternion<f32> = correction.into();
            quat * correction
        } else {
            quat
        };

        let (roll, pitch, yaw) = quat.euler_angles();
        Some((
            quat.into(),
            EulerAngle {
                roll: roll,
                pitch: pitch,
                yaw: yaw,
            },
        ))
    }
    pub fn get_bno055_calib(&self) -> Option<[u8; BNO055_CALIB_SIZE]> {
        self.config
            .clone_inner()
            .map(|conf| {
                conf["sensors"]["bno055"]["calibration"].as_str().map(|s|String::from(s))
            }).flatten()
            .map(|string| -> [u8; 22] { vec_to_calib(hex_decode(string.as_str())) })
    }
    pub fn set_bno055_calib(&self, calib: &[u8; BNO055_CALIB_SIZE]) -> anyhow::Result<()> {
        self.config.open(|config| {
            if let Some(conf) = config {
                conf["sensors"]["bno055"]["calibration"] = value(hex_encode(calib))
            }
        });
        self.update_file()
    }
    pub fn update_file(&self) -> anyhow::Result<()> {
        if let Some(doc) = self.config.clone_inner() {
            fs::write(CONFIG_PATH, doc.to_string())?;
        }

        Ok(())
    }
}
