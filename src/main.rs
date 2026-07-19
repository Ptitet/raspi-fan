use clap::Parser;
use rppal::{
    gpio::Gpio,
    pwm::{Channel, Polarity, Pwm},
};
use serde::Deserialize;
use std::{fs, path::PathBuf, thread, time::Duration};

const DEFAULT_CONFIG_PATH: &str = "/etc/raspi-fan/config.toml";

#[derive(Deserialize)]
struct Config {
    mode: Mode,
    speed: f64,
    bcm_pin: u8,
    pwm_frequency: f64,
    sleep: u64,
    min_temp: f64,
    max_temp: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Auto,
    Manual,
}

impl Config {
    pub fn parse_from_file(path: &PathBuf) -> Self {
        let raw_config = fs::read_to_string(path).expect("Could not open config file");
        toml::from_str(&raw_config).expect("Could not parse config")
    }

    pub fn compute_speed_from_temp(&self, temp: f64) -> f64 {
        let raw_speed = (temp - self.min_temp) / (self.max_temp - self.min_temp);
        raw_speed.clamp(0., 1.)
    }
}

#[derive(Parser)]
#[command(
    name = "raspi-fan",
    version = "0.1.0",
    about = "Manage the Raspberry fan"
)]
struct Daemon {
    #[arg(short, long, value_name = "CONFIG", default_value = DEFAULT_CONFIG_PATH)]
    config: PathBuf,
}

struct FanController(Pwm);
impl FanController {
    fn get_pwm_channel(bcm_pin: u8) -> Option<Channel> {
        match bcm_pin {
            12 | 18 => Some(Channel::Pwm0),
            13 | 19 => Some(Channel::Pwm1),
            _ => None,
        }
    }

    pub fn new(bcm_pin: u8, frequency: f64) -> Result<Self, ()> {
        if let Some(channel) = FanController::get_pwm_channel(bcm_pin) {
            let pwm = Pwm::with_frequency(channel, frequency, 0., Polarity::Normal, false)
                .expect("Could not initialize pwm");
            Ok(FanController(pwm))
        } else {
            Err(()) // invalid pin
        }
    }

    pub fn start(&self) {
        self.0.enable().expect("Could not enable PWM channel");
    }

    pub fn set_speed(&self, speed: f64) {
        self.0.set_duty_cycle(speed).expect("Could not set speed");
    }
}

fn main() {
    let args = Daemon::parse();
    let config = Config::parse_from_file(&args.config);

    let controller = FanController::new(config.bcm_pin, config.pwm_frequency)
        .expect("Invalid bcm pin for hardware pwm");
    controller.start();

    match config.mode {
        Mode::Manual => {
            controller.set_speed(config.speed);
            println!("Set fan speed to {}", config.speed);
        }
        Mode::Auto => loop {
            let cpu_temp = get_cpu_temp();
            let duty_cycle = config.compute_speed_from_temp(cpu_temp);
            controller.set_speed(duty_cycle);
            println!("CPU temp : {cpu_temp}°C, fan speed : {duty_cycle}");
            thread::sleep(Duration::from_secs(config.sleep));
        },
    }
}

fn get_cpu_temp() -> f64 {
    fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .expect("Could not read cpu temperature from sysfs")
        .trim()
        .parse::<u64>()
        .expect("Could not parse CPU temp from sysfs") as f64
        / 1000.
}
