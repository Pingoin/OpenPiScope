use futures::{join, prelude::*};
use generated::open_pi_scope::open_pi_scope_server_server::{
    OpenPiScopeServer, OpenPiScopeServerServer,
};
use generated::open_pi_scope::{
    Broadcast, Constants, GnssDataRequest, GnssDataResponse, MagneticDataRequest,
    MagneticDataResponse, OrientationDataRequest, OrientationDataResponse,
};
use nalgebra::UnitQuaternion;
use prost::Message;
use rppal::i2c::I2c;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::net::UdpSocket;
use tokio_util::codec::{Framed, LinesCodec};
use tonic::transport::Server;
use tonic::Response;

pub(crate) mod helpers;

pub(crate) mod generated {
    use nalgebra::UnitQuaternion;

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

    impl Into<UnitQuaternion<f32>> for open_pi_scope::Quaternion {
        fn into(self) -> UnitQuaternion<f32> {
            UnitQuaternion::new_normalize(nalgebra::Quaternion::new(self.w, self.i, self.j, self.k))
        }
    }

    impl From<UnitQuaternion<f32>> for open_pi_scope::Quaternion {
        fn from(value: UnitQuaternion<f32>) -> Self {
            open_pi_scope::Quaternion {
                w: value.w,
                i: value.i,
                j: value.j,
                k: value.k,
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting");

    static GPS_SYSTEM: storage::Storage = storage::Storage::new();

    let _res = join!(
        handle_gnss(&GPS_SYSTEM),
        handle_rpc(&GPS_SYSTEM),
        handle_broadcasting(&GPS_SYSTEM),
        handle_i2c(&GPS_SYSTEM)
    );
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

async fn handle_i2c(storage: &storage::Storage) -> anyhow::Result<()> {
    let i2c = I2c::with_bus(8).unwrap();

    let mut imu = bno055::Bno055::new(i2c);
    let mut delay = linux_embedded_hal::Delay;

    imu.init(&mut delay)?;

    // Enable 9-degrees-of-freedom sensor fusion mode with fast magnetometer calibration
    imu.set_mode(bno055::BNO055OperationMode::NDOF, &mut delay)?;

    loop {
        let quat = imu.quaternion()?;
        let dec = storage.get_magnetic_data().declination.to_radians();
        // Rotation um Z-Achse
        let declination_rotation =
            UnitQuaternion::from_axis_angle(&nalgebra::Vector3::z_axis(), dec);

        let quat = UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
            quat.s, quat.v.x, quat.v.y, quat.v.z,
        ));
        let quat = quat * declination_rotation;
        let (roll, pitch, yaw) = quat.euler_angles();
        println!(
            "roll: {}°, alt: {}°, AZ: {}°",
            roll.to_degrees(),
            pitch.to_degrees(),
            yaw.to_degrees()
        );
        storage.update_orientation(quat.into());
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn handle_broadcasting(storage: &storage::Storage) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?; // ausgehend, beliebiger Port
    socket.set_broadcast(true)?;

    loop {
        let data = Broadcast {
            magic_number: Constants::MagicNumber as u32,
        }
        .encode_to_vec();
        socket.send_to(&data, "192.168.178.255:12961").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
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
        Ok(Response::new(MagneticDataResponse {
            magnetic_data: Some(self.storage.get_magnetic_data().clone()),
        }))
    }
    async fn get_orientation_data(
        &self,
        _request: tonic::Request<OrientationDataRequest>,
    ) -> Result<tonic::Response<OrientationDataResponse>, tonic::Status> {
        let (quat, euler) = self
            .storage
            .get_orientation()
            .map(|(q, e)| (Some(q), Some(e)))
            .unwrap_or((None, None));
        Ok(Response::new(OrientationDataResponse {
            euler: euler,
            quaternion: quat,
        }))
    }
}
