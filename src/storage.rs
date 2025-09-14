use bno055::BNO055Calibration;
use chrono::Datelike;
use gpsd_proto::UnifiedResponse;
use nalgebra::UnitQuaternion;
use open_pi_scope::{
    alignment::{AlignmentData, EulerAngle, Orientation},
    gnss::{GnssData, Position},
    magnetic::MagneticData,
};
use std::{
    fs,
    sync::{Arc, OnceLock},
};
use tokio::sync::Mutex;
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

use crate::helpers::{hex_decode, hex_encode, vec_to_calib};

const CONFIG_PATH: &str = "/boot/open-pi-scope/config.toml";

pub(crate) fn storage() -> &'static Arc<Storage> {
    static STORAGE: OnceLock<Arc<Storage>> = OnceLock::new();
    STORAGE.get_or_init(|| Arc::new(Storage::new()))
}

#[derive(Debug)]
pub(crate) struct Storage {
    pub(crate) gnss_data: Arc<GnssData>,
    pub(crate) magnetic_data: MagneticData,
    pub(crate) alingment_data: AlignmentData,
    config: Arc<Mutex<DocumentMut>>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            gnss_data: Arc::new(GnssData::default()),
            magnetic_data: MagneticData::default(),
            alingment_data: AlignmentData::default(),
            config: Arc::new(Mutex::new(DocumentMut::new())),
        }
    }
    pub async fn load_config(&self) -> anyhow::Result<()> {
        // Datei einlesen
        let content = fs::read_to_string(CONFIG_PATH)?;
        let doc = content.parse::<DocumentMut>()?;
        // TOML-Dokument parsen (Kommentare bleiben erhalten)
        let mut document = self.config.lock().await;
        *document = doc.clone();
        Ok(())
    }

    pub async fn update_gpsd(&self, line: String) -> Result<(), LinesCodecError> {
        match serde_json::from_str(&line) {
            Ok(rd) => match rd {
                UnifiedResponse::Tpv(t) => {
                    self.gnss_data.set_lat(t.lat.unwrap_or_default()).await;
                    self.gnss_data.set_lon(t.lon.unwrap_or_default()).await;
                    self.gnss_data.set_alt(t.alt.unwrap_or_default()).await;
                    self.gnss_data
                        .set_leap_seconds(t.leapseconds.unwrap_or_default())
                        .await;
                    self.gnss_data
                        .set_estimated_error_altitude(t.epv.unwrap_or_default())
                        .await;
                    self.gnss_data
                        .set_estimated_error_climb(t.epc.unwrap_or_default())
                        .await;
                    self.gnss_data
                        .set_estimated_error_speed(t.eps.unwrap_or_default())
                        .await;
                    self.gnss_data
                        .set_estimated_error_track(t.epd.unwrap_or_default())
                        .await;
                    self.gnss_data.set_speed(t.speed.unwrap_or_default()).await;
                    self.gnss_data.set_track(t.track.unwrap_or_default()).await;
                    self.gnss_data.set_mode(t.mode.into()).await;
                    self.gnss_data.set_climb(t.climb.unwrap_or_default()).await;
                    self.gnss_data
                        .set_estimated_error_plane(t.eph.unwrap_or_default())
                        .await;
                    self.gnss_data
                        .set_estimated_error_latitude(t.epy.unwrap_or_default())
                        .await;
                    self.gnss_data
                        .set_estimated_error_longitude(t.epx.unwrap_or_default())
                        .await;

                    println!(
                        "Fix: {} / Sattelites: {}",
                        t.mode,
                        self.gnss_data.get_satellites().await.len()
                    );

                    self.update_magnetic().await;
                }
                UnifiedResponse::Sky(s) => {
                    if let Some(sats) = s.satellites.clone() {
                        let sats = sats.iter().map(|sat| sat.clone().into()).collect();

                        self.gnss_data.set_satellites(sats).await;
                    }
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

        let now = chrono::Utc::now();

        if let Ok(geomagnetic_field) = GeomagneticField::new(
            Length::new::<meter>(pos.altitude),         // height
            Angle::new::<degree>(pos.latitude as f32),  // lat
            Angle::new::<degree>(pos.longitude as f32), // lon
            Date::from_ordinal_date(now.year(), now.ordinal() as u16).unwrap_or(Date::MIN), // date
        ) {
            self.magnetic_data
                .set_declination(geomagnetic_field.declination().get::<degree>())
                .await;
            self.magnetic_data
                .set_inclination(geomagnetic_field.inclination().get::<degree>())
                .await;
            self.magnetic_data
                .set_magnetic_flux_density(geomagnetic_field.f().get::<microtesla>())
                .await;
        };
    }

    pub async fn get_gnss_data(&self) -> Arc<GnssData> {
        dbg!(&self.gnss_data);
        let bla = self.gnss_data.clone();
        dbg!(&bla);
        bla.clone()
    }
    pub async fn get_magnetic_data(&self) -> MagneticData {
        self.magnetic_data.clone()
    }
    pub async fn get_position(&self) -> Position {
        Position {
            latitude: self.gnss_data.get_lat().await,
            longitude: self.gnss_data.get_lon().await,
            altitude: self.gnss_data.get_alt().await,
        }
    }
    pub async fn update_orientation(&self, orientation: UnitQuaternion<f32>) {
        self.alingment_data.set_alignment(Some(orientation)).await;
    }

    pub async fn get_orientation(&self) -> Option<Orientation> {
        let quat: UnitQuaternion<f32> = self.alingment_data.get_alignment().await?;
        let quat = if let Some(correction) = self.alingment_data.get_correction().await {
            let correction: UnitQuaternion<f32> = correction;
            quat * correction
        } else {
            quat
        };

        let (roll, pitch, yaw) = quat.euler_angles();
        Some(Orientation {
            quaternion: quat.clone(),
            euler: EulerAngle {
                roll: roll.to_degrees(),
                pitch: pitch.to_degrees(),
                yaw: yaw.to_degrees(),
            },
        })
    }
    pub async fn get_bno055_calib(&self) -> Option<BNO055Calibration> {
        let document = self.config.lock().await;

        document["sensors"]["bno055"]["calibration"]
            .as_str()
            .map(|s| String::from(s))
            .map(|string| -> BNO055Calibration { vec_to_calib(hex_decode(string.as_str())) })
    }
    pub async fn set_bno055_calib(&self, calib: BNO055Calibration) -> anyhow::Result<()> {
        let mut document = self.config.lock().await;

        document["sensors"]["bno055"]["calibration"] = value(hex_encode(calib.as_bytes()));
        self.update_file().await
    }
    pub async fn update_file(&self) -> anyhow::Result<()> {
        let document = self.config.lock().await;
        let string = document.to_string();
        fs::write(CONFIG_PATH, string)?;
        Ok(())
    }
}
