use embedded_hal::digital::OutputPin;
use embedded_hal::delay::DelayNs;

#[derive(Debug)]
pub struct Stepper<STEP, DIR, EN> {
    step: STEP,
    dir: DIR,
    enable: Option<EN>,
    max_speed_hz: f32,
    acceleration: f32,
    position: i32,
}

impl<STEP, DIR, EN> Stepper<STEP, DIR, EN>
where
    STEP: OutputPin,
    DIR: OutputPin,
    EN: OutputPin,
{
    /// Erstelle einen neuen Stepper mit optionalem Enable-Pin
    pub fn new(step: STEP, dir: DIR, enable: Option<EN>, max_speed_hz: f32, acceleration: f32) -> Self {
        Self {
            step,
            dir,
            enable,
            max_speed_hz,
            acceleration,
            position: 0,
        }
    } 

    /// Aktiviere den Treiber (falls EN vorhanden)
    pub fn enable(&mut self) {
        if let Some(en) = self.enable.as_mut() {
            let _ = en.set_low(); // DRV8825: LOW = enable
        }
    }

    /// Deaktiviere den Treiber (falls EN vorhanden)
    pub fn disable(&mut self) {
        if let Some(en) = self.enable.as_mut() {
            let _ = en.set_high(); // DRV8825: HIGH = disable
        }
    }

    pub fn step<D: DelayNs>(&mut self, delay: &mut D, steps: i32) {
        if steps> 0 {
            let _ = self.dir.set_low();
        } else {
            let _ = self.dir.set_high();
        }
        let _ = self.dir.set_low();
        self.perform_ramped_steps(delay, steps );
    }
    /// Schritt mit Rampe (sehr einfaches Modell)
    fn perform_ramped_steps<D: DelayNs>(&mut self, delay: &mut D, delta_steps: i32) {
        let steps = delta_steps.abs() as u32;
        let min_delay_us = (1_000_000.0 / self.max_speed_hz) as u32;

        for i in 0..steps {
            let progress = i as f32 / steps as f32;
            let ramp = if progress < 0.5 {
                2.0 * progress
            } else {
                2.0 * (1.0 - progress)
            };
            let adjusted_delay = (min_delay_us as f32 / ramp.max(0.1)) as u32;

            self.pulse_step(delay, adjusted_delay);
            self.position += delta_steps.signum(); // +1 vorwärts, -1 rückwärts
        }
    }

    fn pulse_step<D: DelayNs>(&mut self, delay: &mut D, pulse_width_us: u32) {
        let _ = self.step.set_high();
        let _ = delay.delay_us(pulse_width_us / 2);
        let _ = self.step.set_low();
        let _ = delay.delay_us(pulse_width_us / 2);
    }

        /// Gibt die aktuelle Position zurück
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Setzt die aktuelle Position (z. B. bei Homing)
    pub fn set_position(&mut self, pos: i32) {
        self.position = pos;
    }

    pub fn set_max_speed(&mut self, max_speed_hz: f32) {
        self.max_speed_hz = max_speed_hz;
    }

    pub fn set_acceleration(&mut self, acceleration: f32) {
        self.acceleration = acceleration;
    }

    pub fn get_max_speed(&self) -> f32 {
        self.max_speed_hz
    }

    pub fn get_acceleration(&self) -> f32 {
        self.acceleration
    }

    pub fn step_to_position<D: DelayNs>(&mut self, delay: &mut D, target_position: i32) {
        let delta_steps = target_position - self.position;
        if delta_steps != 0 {
            self.step(delay, delta_steps);
        }
    }
}