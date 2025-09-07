use futures::{join, prelude::*};

use nalgebra::UnitQuaternion;

use open_pi_scope::{Broadcast, MAGIC_NUMBER};
use rppal::i2c::I2c;
use static_cell::StaticCell;
use std::{error::Error, net::SocketAddr, time::Duration};
use tokio::net::{TcpStream, UdpSocket};
use tokio_util::codec::{Framed, LinesCodec};

pub(crate) mod helpers;

mod storage;

static STORAGE: StaticCell<storage::Storage> = StaticCell::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting");

    let store = STORAGE.init(storage::Storage::new());

    store.load_config().await?;

    let _res = join!(
        handle_gnss(store),
        handle_web(store),
        handle_broadcasting(store),
        handle_i2c(store),
        alpaca::handle_alpaca(store),
        alt_az_driver::run_alt_az_driver()
    );
    Ok(())
}

async fn handle_web(_storage: &'static storage::Storage) -> anyhow::Result<()> {
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

    let calib = storage.get_bno055_calib().await;

    if let Some(calib) = calib {
        imu.set_calibration_profile(calib, &mut delay)?;
    }

    loop {
        let quat = imu.quaternion()?;
        let dec = storage.magnetic_data.get_declination().await.to_radians();
        // Rotation um Z-Achse
        let declination_rotation =
            UnitQuaternion::from_axis_angle(&nalgebra::Vector3::z_axis(), dec);

        let quat = UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
            quat.s, quat.v.x, quat.v.y, quat.v.z,
        ));
        let quat = quat * declination_rotation;
        let (_roll, _pitch, _yaw) = quat.euler_angles();
        storage.update_orientation(quat).await;

        let calib = imu.calibration_profile(&mut delay)?;

        storage.set_bno055_calib(calib).await?;

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn handle_broadcasting(_storage: &storage::Storage) -> anyhow::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?; // ausgehend, beliebiger Port
    socket.set_broadcast(true)?;

    loop {
        let data = serde_json::to_vec(&Broadcast {
            magic_number: MAGIC_NUMBER,
        })?;
        socket.send_to(&data, "192.168.178.255:12961").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}


/* #[tonic::async_trait]
impl OpenPiScopeServer for Rpc {
    async fn get_gnss_data(
        &self,
        _request: tonic::Request<GnssDataRequest>,
    ) -> Result<tonic::Response<GnssDataResponse>, tonic::Status> {
        let data = self.storage.get_gnss_data().await;
        println!("GNSS Data: {:?}", &data);
        Ok(tonic::Response::new(GnssDataResponse {
            gnss_data: Some(data.clone()),
        }))
    }

    async fn get_magnetic_data(
        &self,
        _request: tonic::Request<MagneticDataRequest>,
    ) -> Result<tonic::Response<MagneticDataResponse>, tonic::Status> {
        Ok(Response::new(MagneticDataResponse {
            magnetic_data: Some(self.storage.get_magnetic_data().await.clone()),
        }))
    }
    async fn get_orientation_data(
        &self,
        _request: tonic::Request<OrientationDataRequest>,
    ) -> Result<tonic::Response<OrientationDataResponse>, tonic::Status> {
        let (quat, euler) = self
            .storage
            .get_orientation()
            .await
            .map(|(q, e)| (Some(q), Some(e)))
            .unwrap_or((None, None));
        Ok(Response::new(OrientationDataResponse {
            euler: euler,
            quaternion: quat,
        }))
    }
}
 */
mod alpaca;
mod alt_az_driver;
mod stepper_axis;
mod stepper_motor;
pub(crate) mod telescope_position;
