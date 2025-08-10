use crate::mutex_box::MutexBox;

use super::stepper_axis::StepperAxis;
use super::storage;
use super::telescope_position::TelescopePosition;
use tokio::task;

use anyhow::Result;
use linux_embedded_hal::Delay;
use rppal::gpio::Gpio;
use rppal::gpio::OutputPin as RppalOutputPin;
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

        let mut ax = StepperAxis::new(
            alt_step,
            alt_dir,
            Some(alt_en),
            100.0, // steps per unit
            10.0,  // max speed in units/sec
            1.0,   // acceleration in units/sec^2
        );
        let m = MutexBox::new();
        m.set(Some(ax));
        m
    })
}

fn az_axis() -> &'static MutexBox<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>> {
    static AZ_AXIS: OnceLock<
        MutexBox<StepperAxis<RppalOutputPin, RppalOutputPin, RppalOutputPin>>,
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
        let mut ax = StepperAxis::new(
            az_step,
            az_dir,
            Some(az_en),
            100.0, // steps per unit
            10.0,  // max speed in units/sec
            1.0,   // acceleration in units/sec^2
        );

        let m = MutexBox::new();
        m.set(Some(ax));
        m
    })
}

#[derive(Debug, Clone)]
pub struct AltAzDriver {
    storage: &'static storage::Storage,
}

impl AltAzDriver {
    pub fn new(storage: &'static storage::Storage) -> Result<Self> {
        Ok(AltAzDriver { storage })
    }

    pub async fn set_target_position(&self, target: TelescopePosition) -> Result<()> {
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
        Ok(())
    }
}
