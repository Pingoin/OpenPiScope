use chrono::Datelike;
use gpsd_proto::UnifiedResponse;

use tokio_util::codec::LinesCodecError;
use world_magnetic_model::{time::Date, uom::si::{angle::{degree, Angle}, f32::Length, length::meter, magnetic_flux_density::microtesla}, GeomagneticField};

use crate::{
    generated::open_pi_scope::{GnssData, MagneticData, Position},
    helpers::MutexBox,
};

pub(crate) struct Storage {
    pub(crate) gnss_data: MutexBox<GnssData>,
    pub(crate) magnetic_data: MutexBox<MagneticData>,
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
        }
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
                        println!("Fix: {} / Sattelites: {}",t.mode,data.satellites.len());
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
                Length::new::<meter>(pos.altitude), // height
                Angle::new::<degree>(pos.latitude as f32), // lat
                Angle::new::<degree>(pos.longitude as f32), // lon
                Date::from_ordinal_date(now.year(), now.ordinal()as u16).unwrap_or(Date::MIN) // date
            ){
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
}
