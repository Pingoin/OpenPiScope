use super::stepper_motor::Stepper;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

#[derive(Debug)]
pub struct StepperAxis<STEP, DIR, EN> {
    stepper: Stepper<STEP,DIR, EN>,
    steps_per_unit: f32,
}

impl<STEP, DIR, EN> StepperAxis<STEP, DIR, EN>
where
    STEP: OutputPin,
    DIR: OutputPin,
    EN: OutputPin,
{
    pub fn new(step: STEP, dir: DIR, enable: Option<EN>, steps_per_unit: f32,max_speed_units_per_sec:f32, acceleration:f32) -> Self {

        
        let stepper = Stepper::new(step, dir, enable, max_speed_units_per_sec/steps_per_unit, acceleration/steps_per_unit);

        Self {
            stepper,
            steps_per_unit,
        }
    }

    /// Enables the stepper motor
    /// This will set the enable pin to low (if available)
    pub fn enable(&mut self) {
        self.stepper.enable();
    }

   /// Disables the stepper motor
   /// This will set the enable pin to high (if available)
    pub fn disable(&mut self) {
        self.stepper.disable();
    }

    
    pub fn step_to_position<D: DelayNs>(&mut self, delay: &mut D, target_position: f32) {
        let target_steps = (target_position * self.steps_per_unit) as i32;
        self.stepper.step_to_position(delay, target_steps);
    }

    pub fn position(&self) -> f32 {
        self.stepper.position() as f32 / self.steps_per_unit
    }

    pub fn set_position(&mut self, pos: f32) {
        let pos_steps = (pos * self.steps_per_unit) as i32;
        self.stepper.set_position(pos_steps);
    }

    pub fn set_max_speed(&mut self, max_speed_units_per_sec: f32) {
        self.stepper.set_max_speed(max_speed_units_per_sec / self.steps_per_unit);
    }

    pub fn set_acceleration(&mut self, acceleration_units_per_sec: f32) {
        self.stepper.set_acceleration(acceleration_units_per_sec / self.steps_per_unit);
    }

    pub fn max_speed(&self) -> f32 {
        self.stepper.get_max_speed() * self.steps_per_unit
    }
    pub fn acceleration(&self) -> f32 {
        self.stepper.get_acceleration() * self.steps_per_unit
    }


}