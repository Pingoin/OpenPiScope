use derivative::Derivative;
use futures::prelude::*;
use gpsd_proto::{Mode, Satellite, UnifiedResponse};
use std::{cell::RefCell, error::Error};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting");

    let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();

    let stream = TcpStream::connect(&addr).await?;
    let mut framed: Framed<TcpStream, LinesCodec> = Framed::new(stream, LinesCodec::new());
    let gps_system = GpsSystem::new();

    framed.send(gpsd_proto::ENABLE_WATCH_CMD).await?;
    framed.try_for_each(|line| gps_system.update(line)).await?;

    Ok(())
}

struct GpsSystem {
    data: critical_section::Mutex<RefCell<GpsData>>,
}

impl GpsSystem {
    pub fn new() -> Self {
        GpsSystem {
            data: critical_section::Mutex::new(RefCell::new(GpsData::default())),
        }
    }

    pub async fn update(&self, line: String) -> Result<(), LinesCodecError> {
        match serde_json::from_str(&line) {
            Ok(rd) => match rd {
                UnifiedResponse::Tpv(t) => {
                    critical_section::with(|cs| {
                        let mut data = self.data.borrow(cs).borrow_mut();
                        data.lat = t.lat;
                        data.lon = t.lon;
                        data.alt = t.alt;
                        data.leap_seconds = t.leapseconds;
                        data.estimated_error_longitude = t.epx;
                        data.estimated_error_latitude = t.epy;
                        data.estimated_error_plane = t.eph;
                        data.estimated_error_altitude = t.epv;
                        data.track = t.track;
                        data.speed = t.speed;
                        data.mode = t.mode;
                        data.climb = t.climb;
                        data.estimated_error_track = t.epd;
                        data.estimated_error_speed = t.eps;
                        data.estimated_error_climb = t.epc;
                        dbg!(data.satellites.len());
                    });
                },
                UnifiedResponse::Sky(s) => {
                    critical_section::with(|cs| {
                        let mut data = self.data.borrow(cs).borrow_mut();
                        if let Some(sats) = s.satellites.clone() {
                            data.satellites = sats;
                        }
                        println!("Sky: {:?}/ sats: {}",s.device, s.satellites.unwrap_or_default().len());
                    });
                },
                _ => {}
            },
            Err(e) => {
                println!("Error decoding: {e}");
            }
        };
        Ok(())
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
struct GpsData {
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub alt: Option<f32>,
    pub leap_seconds: Option<i32>,
    pub estimated_error_longitude: Option<f32>,
    pub estimated_error_latitude: Option<f32>,
    pub estimated_error_plane: Option<f32>,
    pub estimated_error_altitude: Option<f32>,
    pub track: Option<f32>,
    pub speed: Option<f32>,
    pub climb: Option<f32>,
    #[derivative(Default(value = "Mode::NoFix"))]
    pub mode: Mode,
    pub estimated_error_track: Option<f32>,
    pub estimated_error_speed: Option<f32>,
    pub estimated_error_climb: Option<f32>,
    pub satellites: Vec<Satellite>,
}

