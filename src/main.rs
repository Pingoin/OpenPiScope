use futures::{join, prelude::*};
use generated::open_pi_scope::open_pi_scope_server_server::{OpenPiScopeServer, OpenPiScopeServerServer};
use generated::open_pi_scope::{GnssDataRequest, GnssDataResponse, MagneticDataRequest, MagneticDataResponse};
use tonic::Response;
use std::net::SocketAddr;
use std::error::Error;
use tokio::net::TcpStream;
use tokio_util::codec::{Framed, LinesCodec};
use tonic::transport::Server;

pub(crate) mod helpers;

pub(crate) mod generated {
    pub(crate) mod open_pi_scope;

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!("generated/reflection.bin");

    impl From<gpsd_proto::Satellite> for open_pi_scope::Satellite {
        fn from(value: gpsd_proto::Satellite) -> Self {
            open_pi_scope::Satellite {
                prn: value.prn as i32,
                elevation: value.el.unwrap_or_default(),
                azimuth: value.az.unwrap_or_default(),
                signal_strength: value.ss.unwrap_or_default(),
                used: value.used,
                system: value.gnssid.unwrap_or_default() as i32,
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting");
    static GPS_SYSTEM: storage::Storage = storage::Storage::new();

    let _res = join!(handle_gnss(&GPS_SYSTEM), handle_rpc(&GPS_SYSTEM));
    Ok(())
}

async fn handle_rpc(gps_system: &'static storage::Storage) -> anyhow::Result<()> {
    let addr = "0.0.0.0:50051".parse()?;

    let rpc = Rpc {
        storage: gps_system,
    };
    let reflection_1 = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(generated::FILE_DESCRIPTOR_SET)
        .build_v1()?;
    let reflection_1a = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(generated::FILE_DESCRIPTOR_SET)
        .build_v1alpha()?;

    Server::builder()
        .add_service(OpenPiScopeServerServer::new(rpc))
        .add_service(reflection_1)
        .add_service(reflection_1a)
        .serve(addr)
        .await?;

    Ok(())
}

async fn handle_gnss(gps: &storage::Storage) -> anyhow::Result<()> {
    let addr: SocketAddr = "127.0.0.1:2947".parse().unwrap();
    let stream = TcpStream::connect(&addr).await?;
    let mut framed: Framed<TcpStream, LinesCodec> = Framed::new(stream, LinesCodec::new());
    framed.send(gpsd_proto::ENABLE_WATCH_CMD).await?;
    framed.try_for_each(|line| gps.update_gpsd(line)).await?;
    Ok(())
}

mod storage;

struct Rpc {
    storage: &'static storage::Storage,
}

#[tonic::async_trait]
impl OpenPiScopeServer for Rpc {
    async fn get_gnss_data(
        &self,
        _request: tonic::Request<GnssDataRequest>,
    ) -> Result<tonic::Response<GnssDataResponse>, tonic::Status> {
        let data = self.storage.get_gnss_data();

        Ok(tonic::Response::new(GnssDataResponse {
            gnss_data: Some(data.clone()),
        }))
    }

    async fn get_magnetic_data(
        &self,
        _request: tonic::Request<MagneticDataRequest>,
    ) -> Result<tonic::Response<MagneticDataResponse>, tonic::Status> {


        Ok(Response::new(MagneticDataResponse{
            magnetic_data: Some(self.storage.get_magnetic_data().clone()),
        }))
    }
}
