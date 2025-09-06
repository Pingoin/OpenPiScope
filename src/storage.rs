use bno055::BNO055Calibration;
use chrono::Datelike;
use gpsd_proto::UnifiedResponse;
use nalgebra::UnitQuaternion;
use std::fs;
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
        AlignmentData, EulerAngle, GnssData, MagneticData,  Position, Quaternion,
    },
    helpers::{hex_decode, hex_encode, vec_to_calib},
    mutex_box::MutexBox,
};

const CONFIG_PATH: &str = "/boot/open-pi-scope/config.toml";

#[derive(Debug)]
pub(crate) struct Storage {
    gnss_data: MutexBox<GnssData>,
    magnetic_data: MutexBox<MagneticData>,
    alingment_data: MutexBox<AlignmentData>,
    config: MutexBox<DocumentMut>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            gnss_data: MutexBox::new(),
            magnetic_data: MutexBox::new(),
            alingment_data: MutexBox::new(),
            config: MutexBox::new(),
        }
    }
    pub async fn load_config(&self) -> anyhow::Result<()> {
        // Datei einlesen
        let content = fs::read_to_string(CONFIG_PATH)?;
        let doc = content.parse::<DocumentMut>()?;
        // TOML-Dokument parsen (Kommentare bleiben erhalten)
        self.config.set(Some(doc.clone())).await;
        self.gnss_data.set(Some(GnssData::default())).await;
        self.magnetic_data.set(Some(MagneticData::default())).await;
        self.alingment_data
            .set(Some(AlignmentData::default()))
            .await;
        Ok(())
    }

    pub async fn update_gpsd(&self, line: String) -> Result<(), LinesCodecError> {
        match serde_json::from_str(&line) {
            Ok(rd) => match rd {
                UnifiedResponse::Tpv(t) => {
                    self.gnss_data
                        .open(|mut data| {
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
                            data.mode = t.mode.into();
                            data.climb = t.climb.unwrap_or_default();
                            data.estimated_error_track = t.epd.unwrap_or_default();
                            data.estimated_error_speed = t.eps.unwrap_or_default();
                            data.estimated_error_climb = t.epc.unwrap_or_default();
                            println!("Fix: {} / Sattelites: {}", t.mode, data.satellites.len());
                            (data, ())
                        })
                        .await;
                    self.update_magnetic().await;
                }
                UnifiedResponse::Sky(s) => {
                    self.gnss_data
                        .open(|mut data| {
                            if let Some(sats) = s.satellites.clone() {
                                data.satellites =
                                    sats.iter().map(|sat| sat.clone().into()).collect();
                            }
                            (data, ())
                        })
                        .await;
                }
                _ => {}
            },
            Err(e) => {
                println!("Error decoding: {e}");
            }
        };
        Ok(())
    }
    async fn update_magnetic(&self) {
        let pos = self.get_position().await;
        self.magnetic_data
            .open(|mut mag| {
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
                (mag, ())
            })
            .await;
    }
    pub async fn get_gnss_data(&self) -> GnssData {
        let gnss_data = self.gnss_data.clone_inner().await;

        gnss_data.unwrap_or_default()
    }
    pub async fn get_magnetic_data(&self) -> MagneticData {
        self.magnetic_data.clone_inner().await.unwrap_or_default()
    }
    pub async fn get_position(&self) -> Position {
        self.gnss_data
            .open(|gnss| {
                (
                    gnss.clone(),
                    Position {
                        latitude: gnss.lat,
                        longitude: gnss.lon,
                        altitude: gnss.alt,
                    },
                )
            })
            .await
            .unwrap_or_default()
    }
    pub async fn update_orientation(&self, orientation: Quaternion) {
        self.alingment_data
            .open(|mut alignment| {
                alignment.alignment = Some(orientation);
                (alignment, ())
            })
            .await;
    }
    pub async fn get_orientation(&self) -> Option<(Quaternion, EulerAngle)> {
        let alignment = self.alingment_data.clone_inner().await?;
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
    pub async fn get_bno055_calib(&self) -> Option<BNO055Calibration> {
        self.config
            .clone_inner()
            .await
            .map(|conf| {
                conf["sensors"]["bno055"]["calibration"]
                    .as_str()
                    .map(|s| String::from(s))
            })
            .flatten()
            .map(|string| -> BNO055Calibration { vec_to_calib(hex_decode(string.as_str())) })
    }
    pub async fn set_bno055_calib(&self, calib: BNO055Calibration) -> anyhow::Result<()> {
        self.config
            .open(|mut config| {
                config["sensors"]["bno055"]["calibration"] = value(hex_encode(calib.as_bytes()));

                (config, ())
            })
            .await;
        self.update_file().await
    }
    pub async fn update_file(&self) -> anyhow::Result<()> {
        if let Some(doc) = self.config.clone_inner().await {
            fs::write(CONFIG_PATH, doc.to_string())?;
        }

        Ok(())
    }
}
