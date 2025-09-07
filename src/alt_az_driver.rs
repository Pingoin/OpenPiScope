use super::{stepper_axis::StepperAxis, telescope_position::TelescopePosition};
use crate::storage::storage;
use atomic_struct_core::AtomicMember;
use tokio::{sync::Mutex, task};

use anyhow::Result;
use rppal::gpio::{Gpio, OutputPin as RppalOutputPin};
use std::sync::{Arc, OnceLock};

pub fn alt_az_driver() -> &'static AltAzDriver {
    static ALT_AZ_DRIVER: OnceLock<AltAzDriver> = OnceLock::new();
    ALT_AZ_DRIVER.get_or_init(|| {
        let m = AltAzDriver::new_raw();
        m
    })
}

fn alt_axis() -> &'static Arc<Mutex<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>> {
    static ALT_AXIS: OnceLock<
        Arc<Mutex<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>>,
    > = OnceLock::new();
    ALT_AXIS.get_or_init(|| {
        let gpio = Gpio::new().expect("Failed to initialize GPIO");
        let alt_step = gpio
            .get(17)
            .expect("Failed to get GPIO pin 17")
            .into_output();
        let alt_dir = gpio
            .get(27)
            .expect("Failed to get GPIO pin 27")
            .into_output();
        let alt_en = gpio
            .get(22)
            .expect("Failed to get GPIO pin 22")
            .into_output();

        let ax = StepperAxis::new(
            alt_step,
            alt_dir,
            Some(alt_en),
            100.0, // steps per unit
            10.0,  // max speed in units/sec
            1.0,   // acceleration in units/sec^2
        );

        let m = Arc::new(Mutex::new(ax));
        m
    })
}

fn az_axis() -> &'static Arc<Mutex<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>> {
    static AZ_AXIS: OnceLock<
        Arc<Mutex<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>>,
    > = OnceLock::new();
    AZ_AXIS.get_or_init(|| {
        let gpio = Gpio::new().expect("Failed to initialize GPIO");
        let az_step = gpio
            .get(18)
            .expect("Failed to get GPIO pin 18")
            .into_output();
        let az_dir = gpio
            .get(24)
            .expect("Failed to get GPIO pin 24")
            .into_output();
        let az_en = gpio.get(4).expect("Failed to get GPIO pin 4").into_output();
        let ax = StepperAxis::new(
            az_step,
            az_dir,
            Some(az_en),
            100.0, // steps per unit
            10.0,  // max speed in units/sec
            1.0,   // acceleration in units/sec^2
        );
        let m = Arc::new(Mutex::new(ax));
        m
    })
}

#[atomic_struct::atomic_struct]
#[derive(Debug, Clone)]
pub(crate) struct AltAzDriver {
    pub(crate) target_position: Option<TelescopePosition>,
    position_set: bool,
}

impl AltAzDriver {
    pub fn new_raw() -> Self {
        AltAzDriver {
            target_position: AtomicMember::new(None),
            position_set: AtomicMember::new(false), // position_set
        }
    }

    pub async fn get_current_position(&self) -> Result<TelescopePosition> {
        let alt = alt_axis().lock().await.position();
        let az = az_axis().lock().await.position();
        Ok(TelescopePosition::new_alt_az(alt, az))
    }
    async fn go_to_target_position(&self) -> Result<()> {
        let target = self.target_position.get().await;
        if let Some(target) = target {
            let alt_az_target = target.get_alt_az();
            let alt = alt_az_target.alt;
            let az = alt_az_target.az;
            let handle1 = task::spawn(async move {
                let mut alt_axis_handle = alt_axis().lock().await;
                alt_axis_handle.set_position(alt);
            });
            let handle2 = task::spawn(async move {
                let mut az_axis_handler = az_axis().lock().await;
                az_axis_handler.set_position(az.clone());
            });

            // Auf beide warten
            let _ = tokio::join!(handle1, handle2);
        }
        Ok(())
    }

    async fn set_current_position(&self, position: TelescopePosition) {
        let position = position.get_alt_az();
        let mut alt_axis_handle = alt_axis().lock().await;
        alt_axis_handle.set_position(position.alt);
        let mut az_axis_handle = az_axis().lock().await;
        az_axis_handle.set_position(position.az);
    }
}

pub(crate) async fn run_alt_az_driver() -> Result<()> {
    let driver_handle = alt_az_driver(); // Initialize the AltAz driver

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let orientation = storage().get_orientation().await;

        if let Some(orientation) = orientation {
            if !driver_handle.get_position_set().await {
                let target = TelescopePosition::new_alt_az(
                    orientation.1.pitch as f32,
                    orientation.1.yaw as f32,
                );
                driver_handle.set_current_position(target).await;
                driver_handle.set_position_set(true).await;
            }
        }
        driver_handle.go_to_target_position().await?;
    }
}
