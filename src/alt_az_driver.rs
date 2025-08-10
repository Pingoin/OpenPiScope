use super::{
    mutex_box::MutexBox, stepper_axis::StepperAxis, storage, telescope_position::TelescopePosition,
};
use tokio::task;

use anyhow::Result;
use rppal::gpio::{Gpio, OutputPin as RppalOutputPin};
use std::sync::OnceLock;

pub fn alt_az_driver() -> &'static MutexBox<AltAzDriver> {
    static ALT_AZ_DRIVER: OnceLock<MutexBox<AltAzDriver>> = OnceLock::new();
    ALT_AZ_DRIVER.get_or_init(|| {
        let m = MutexBox::new();
        m
    })
}

fn alt_axis() -> &'static MutexBox<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>> {
    static ALT_AXIS: OnceLock<
        MutexBox<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>,
    > = OnceLock::new();
    ALT_AXIS.get_or_init(|| {
        let m = MutexBox::new();
        m
    })
}

fn az_axis() -> &'static MutexBox<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>> {
    static AZ_AXIS: OnceLock<
        MutexBox<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>,
    > = OnceLock::new();
    AZ_AXIS.get_or_init(|| {
        let m = MutexBox::new();
        m
    })
}

#[derive(Debug, Clone)]
pub struct AltAzDriver {
    storage: &'static storage::Storage,
    target_position: Option<TelescopePosition>,
    position_set: bool,

}

impl AltAzDriver {
    pub fn new(storage: &'static storage::Storage) -> Result<Self> {
        Ok(AltAzDriver {
            storage,
            target_position:None,
            position_set:false, // position_set
        })
    }
    pub async fn set_target_position(&mut self, target: TelescopePosition) -> Result<()> {
        self.target_position =Some(target);
        Ok(())
    }
    pub async fn get_current_position(&self) -> Result<TelescopePosition> {
        let alt_axis = alt_axis()
            .open(|axis| {
                let pos = axis.position();
                (axis, pos)
            })
            .await;
        let az_axis = az_axis()
            .open(|axis| {
                let pos = axis.position();
                (axis, pos)
            })
            .await;

        if let (Some(alt), Some(az)) = (alt_axis, az_axis) {
            let alt_az_position = TelescopePosition::new_alt_az(alt, az);
            Ok(alt_az_position)
        } else {
            Err(anyhow::anyhow!("Failed to get current position from axes"))
        }
    }
    async fn go_to_target_position(&self) -> Result<()> {
        let target = self.target_position.clone();
        if let Some(target) = target {
            let alt_az_target = target.get_alt_az();
            let alt = alt_az_target.alt;
            let az = alt_az_target.az;
            let handle1 = task::spawn(alt_axis().open_async(async move |mut alt_axis| {
                alt_axis.set_position(alt.clone());
                (alt_axis, ())
            }));
            let handle2 = task::spawn(az_axis().open_async(async move |mut az_axis| {
                az_axis.set_position(az.clone());
                (az_axis, ())
            }));

            // Auf beide warten
            let _ = tokio::join!(handle1, handle2);
        }
        Ok(())
    }
    async fn set_current_position(&self, position: TelescopePosition){
        let position= position.get_alt_az();

        alt_axis()
            .open(move |mut alt_axis| {
                alt_axis.set_position(position.alt);
                (alt_axis, ())
            })
            .await;

        az_axis()
            .open(move |mut az_axis| {
                az_axis.set_position(position.az);
                (az_axis, ())
            })
            .await;
    }

}

pub(crate) async fn run_alt_az_driver(storage: &'static storage::Storage) -> Result<()> {

    let driver_handle = alt_az_driver(); // Initialize the AltAz driver
    let driver = AltAzDriver::new(storage)?;
    driver_handle.set(Some(driver)).await;

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

    alt_axis().set(Some(ax)).await;

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

    az_axis().set(Some(ax)).await;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let _ = alt_az_driver()
            .open_async(async |mut driver| {
                let orientation = driver.storage.get_orientation().await;
             
                if let Some(orientation)  =  orientation{
                    if !driver.position_set {
                        let target = TelescopePosition::new_alt_az(
                            orientation.1.pitch as f32,
                            orientation.1.yaw as f32,
                        );
                        driver.set_current_position(target).await;
                        driver.position_set = true;
                    }
                }
                let result = driver.go_to_target_position().await;
                (driver, result)
            })
            .await;
    }
}
