use futures::{join, prelude::*};
use generated::open_pi_scope::gnss_data_server_server::{GnssDataServer, GnssDataServerServer};
use generated::open_pi_scope::{GnssData, GnssDataRequest, GnssDataResponse};
use gpsd_proto::UnifiedResponse;
use tonic::transport::Server;
use std::net::SocketAddr;
use std::{cell::RefCell, error::Error};
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

pub(crate) mod generated {
    pub(crate) mod open_pi_scope;

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("generated/reflection.bin");

    impl From<gpsd_proto::Satellite> for open_pi_scope::Satellite {
        fn from(value: gpsd_proto::Satellite) -> Self {
            open_pi_scope::Satellite {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting");
    static gps_system:GpsSystem = GpsSystem::new();

    handle_gnss(&gps_system).await?;
    join!(handle_gnss(&gps_system),handle_rpc(&gps_system));
    Ok(())
}

async fn handle_rpc(gps_system: &'static GpsSystem) -> anyhow::Result<()> {
    let addr = "[::1]:50051".parse()?;

    let rpc = Rpc { gnss: gps_system };
    let reflection_1 = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(generated::FILE_DESCRIPTOR_SET)
        .build_v1()?;
    let reflection_1a = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(generated::FILE_DESCRIPTOR_SET)
        .build_v1alpha()?;

    Server::builder()
        .add_service(GnssDataServerServer::new(rpc))
        .add_service(reflection_1)
        .add_service(reflection_1a)
        .serve(addr)
        .await?;

    Ok(())
}

async fn handle_gnss(gps: &GpsSystem) -> anyhow::Result<()> {
    let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();
    let stream = TcpStream::connect(&addr).await?;
    let mut framed: Framed<TcpStream, LinesCodec> = Framed::new(stream, LinesCodec::new());
    framed.send(gpsd_proto::ENABLE_WATCH_CMD).await?;
    framed.try_for_each(|line| gps.update(line)).await?;
    Ok(())
}

struct GpsSystem {
    data: critical_section::Mutex<RefCell<GnssData>>,
}

impl GpsSystem {
    pub const fn new() -> Self {
        GpsSystem {
            data: critical_section::Mutex::new(RefCell::new(GnssData{
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
            })),
        }
    }

    pub async fn update(&self, line: String) -> Result<(), LinesCodecError> {
        match serde_json::from_str(&line) {
            Ok(rd) => match rd {
                UnifiedResponse::Tpv(t) => {
                    critical_section::with(|cs| {
                        let mut data = self.data.borrow(cs).borrow_mut();
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
                        dbg!(data.satellites.len());
                    });
                }
                UnifiedResponse::Sky(s) => {
                    critical_section::with(|cs| {
                        let mut data = self.data.borrow(cs).borrow_mut();
                        if let Some(sats) = s.satellites.clone() {
                            data.satellites = sats.iter().map(|sat| sat.clone().into()).collect();
                        }
                        println!(
                            "Sky: {:?}/ sats: {}",
                            s.device,
                            s.satellites.unwrap_or_default().len()
                        );
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
}

struct Rpc {
    gnss: &'static GpsSystem,
}

#[tonic::async_trait]
impl generated::open_pi_scope::gnss_data_server_server::GnssDataServer for Rpc {
    async fn get_gnss_data(
        &self,
        request: tonic::Request<GnssDataRequest>,
    ) -> Result<tonic::Response<GnssDataResponse>, tonic::Status> {
        let data = GnssData::default();
        //let data = self.gnss.data.lock().await.borrow();
        Ok(tonic::Response::new(GnssDataResponse {
            gnss_data: Some(data.clone()),
        }))
    }
}
