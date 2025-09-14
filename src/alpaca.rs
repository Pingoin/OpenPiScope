use ascom_alpaca::api::{AlignmentMode, Device, Telescope};
use ascom_alpaca::{ ASCOMResult};
use async_trait::async_trait;
use  crate::alt_az_driver::alt_az_driver;
use crate::telescope_position::TelescopePosition;

use crate::storage;

pub(crate) async fn handle_alpaca(storage: &'static storage::Storage,) -> anyhow::Result<()> {
    let mut server = ascom_alpaca::Server {
        // helper macro to populate server information from your own Cargo.toml
        info: ascom_alpaca::api::CargoServerInfo!(),
        ..Default::default()
    };

    // By default, the server will listen on dual-stack (IPv4 + IPv6) unspecified address with a randomly assigned port.
    // You can change that by modifying the `listen_addr` field:
    server.listen_addr.set_port(8000);

    // Create and register your device(s).
    server.devices.register(AlpacaTelescope { storage});

    // Start the infinite server loop.
    server
        .start()
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}

#[derive(Debug)]
struct AlpacaTelescope {
    storage: &'static storage::Storage,
}

#[async_trait]
impl Device for AlpacaTelescope {
    fn static_name(&self) -> &str {
        "OpenPiScope Telescope"
    }

    fn unique_id(&self) -> &str {
        "insert GUID here"
    }
    async fn description(&self) -> ASCOMResult<String> {
        Ok("OpenPiScope Telescope Device".to_owned())
    }

    async fn connected(&self) -> ASCOMResult<bool> {
        Ok(true) // Replace with actual connection logic
    }

    async fn set_connected(&self, _connected: bool) -> ASCOMResult {
        Ok(()) // Replace with actual connection logic
    }

    async fn driver_info(&self) -> ASCOMResult<String> {
        Ok("ascom-alpaca Rust webcam demo".to_owned())
    }

    async fn driver_version(&self) -> ASCOMResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_owned())
    }
}

#[async_trait]
impl Telescope for AlpacaTelescope {

    async fn alignment_mode(&self) -> ASCOMResult<AlignmentMode> {
        Ok(AlignmentMode::AltAz)
    }

    async fn slewing(&self) -> ASCOMResult<bool> {
        Ok(false) // Replace with actual slewing status
    }
    async fn slew_to_alt_az(&self, azimuth: f64, altitude: f64) -> ASCOMResult<()> {
        // Implement the logic to slew to the specified azimuth and altitude
        println!("Slewing to Azimuth: {}, Altitude: {}", azimuth, altitude);
        let target = TelescopePosition::new_alt_az(altitude as f32, azimuth as f32);
        alt_az_driver().set_target_position(Some(target)).await;

        Ok(())
    }

    async fn abort_slew(&self) -> ASCOMResult<()> {
        // Implement the logic to abort the current slew operation
        println!("Aborting slew operation");
        Ok(())
    }


    async fn right_ascension(&self) -> ASCOMResult<f64> {
        // Replace with actual logic to get the right ascension
        Ok(0.0)
    }
    async fn declination(&self) -> ASCOMResult<f64> {
        // Replace with actual logic to get the declination
        Ok(0.0)
    }

    async fn azimuth(&self) -> ASCOMResult<f64> {
        let orientation = self.storage.get_orientation().await.unwrap_or_default();
        let azimuth = (orientation.euler.yaw as f64).to_degrees() + 180.0; // Adjusting to 0-360 range
        if azimuth >= 360.0 {
            return Ok(azimuth - 360.0);
        }
        Ok(azimuth)
    }

    async fn altitude(&self) -> ASCOMResult<f64> {
        let orientation = self.storage.get_orientation().await.unwrap_or_default();
        let altitude = orientation.euler.pitch as f64;
        Ok(altitude.to_degrees())
    }

    async fn site_elevation(&self) -> ASCOMResult<f64> {
        let position = self.storage.get_position().await;
        Ok(position.altitude.into())
    }

    async fn site_latitude(&self) -> ASCOMResult<f64> {
        let position = self.storage.get_position().await;
        Ok(position.latitude)
    }

    async fn site_longitude(&self) -> ASCOMResult<f64> {
        let position = self.storage.get_position().await;
        Ok(position.longitude)
    }
    async fn utc_date(&self) -> ASCOMResult<std::time::SystemTime> {
        Ok(std::time::SystemTime::now())
    }
    async fn can_slew_alt_az(&self) -> ASCOMResult<bool> {
        Ok(true) // Replace with actual logic to determine if slewing to Alt/Az is supported
    }
    async fn can_slew_alt_az_async(&self) -> ASCOMResult<bool> {
        Ok(true) // Replace with actual logic to determine if async slewing to Alt/Az is supported
    }

    async fn can_sync_alt_az(&self) -> ASCOMResult<bool> {
        Ok(true) // Replace with actual logic to determine if syncing to Alt/Az is supported
    }

    async fn set_site_elevation(&self, site_elevation: f64) -> ASCOMResult<()> {
        // Implement the logic to set the site elevation
        println!("Setting site elevation to: {}", site_elevation);
        Ok(())
    }
    async fn set_site_latitude(&self, site_latitude: f64) -> ASCOMResult<()> {
        // Implement the logic to set the site latitude
        println!("Setting site latitude to: {}", site_latitude);
        Ok(())
    }
    async fn set_site_longitude(&self, site_longitude: f64) -> ASCOMResult<()> {
        // Implement the logic to set the site longitude
        println!("Setting site longitude to: {}", site_longitude);
        Ok(())
    }

    async fn at_park(&self) -> ASCOMResult<bool> {
        // Implement the logic to check if the telescope is parked
        Ok(false) // Replace with actual parked status
    }
    
    async fn at_home(&self) -> ASCOMResult<bool> {
        // Implement the logic to check if the telescope is parked
        Ok(false) // Replace with actual parked status
    }
}
