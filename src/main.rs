use rppal::pwm::{Channel, Polarity, Pwm};
use serde::{Deserialize, Serialize};
use std::{fs, thread, time::Duration};

const CONFIG_PATH: &str = "/etc/raspifan/config.toml";

#[derive(Serialize, Deserialize)]
enum FanMode {
    Auto,
    Manual(usize),
}

#[derive(Serialize, Deserialize)]
struct Config {
    mode: FanMode,
    bcm_pin: u8,
    frequency: f64,
    sleep: u64,
    min_speed: f64,
    max_speed: f64,
    max_temp: f64,
    min_temp: f64,
}

fn get_cpu_temp() -> f64 {
    fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .expect("Could not read CPU temperature")
        .parse::<f64>()
        .expect("Could not parse CPU temperature")
        / 1000.
}

impl Config {
    pub fn compute_fan_duty_cycle(&self, cpu_temp: f64) -> f64 {
        let raw_duty_cycle = (cpu_temp - self.min_temp) / (self.max_temp - self.min_temp);
        raw_duty_cycle.clamp(self.min_speed, self.max_speed)
    }
}

fn main() {
    let raw_config = fs::read_to_string(CONFIG_PATH).expect("Could not read config file");
    let config = toml::from_str::<Config>(&raw_config).expect("Failed to parse config");

    let pwm_channel = match config.bcm_pin {
        12 | 18 => Channel::Pwm0,
        13 | 19 => Channel::Pwm1,
        _ => panic!("BCM pin {} is not valid for hardware PWM", config.bcm_pin),
    };

    // let gpio = Gpio::new().expect("Could not create a GPIO instance");
    // let mut pin = gpio
    //     .get(config.bcm_pin)
    //     .unwrap_or_else(|_| panic!("Should be able to open BCM pin {}", config.bcm_pin));

    let pwm = Pwm::with_frequency(
        pwm_channel,
        config.frequency,
        config.min_speed,
        Polarity::Normal,
        true,
    )
    .expect("Could not create PWM instance");

    loop {
        let cpu_temp = get_cpu_temp();
        let duty_cycle = config.compute_fan_duty_cycle(cpu_temp);
        pwm.set_duty_cycle(duty_cycle)
            .expect("Failed to set duty cycle");
        thread::sleep(Duration::from_secs(config.sleep));
    }
}
