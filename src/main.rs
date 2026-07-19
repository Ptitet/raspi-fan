use clap::Parser;
use rppal::gpio::Gpio;
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
        (temp - self.min_temp) / (self.max_temp - self.min_temp).clamp(0., 1.)
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

fn main() {
    let args = Daemon::parse();
    let config = Config::parse_from_file(&args.config);

    let gpio = Gpio::new().expect("Could not create GPIO instance");
    let mut pin = gpio
        .get(config.bcm_pin)
        .expect("Could not retrieve pin")
        .into_output();
    match config.mode {
        Mode::Manual => {
            pin.set_pwm_frequency(config.pwm_frequency, config.speed)
                .expect("Could not set fan speed");
            println!("Set fan speed to {}", config.speed);
        }
        Mode::Auto => loop {
            let cpu_temp = get_cpu_temp();
            let duty_cycle = config.compute_speed_from_temp(cpu_temp);
            pin.set_pwm_frequency(config.pwm_frequency, duty_cycle)
                .expect("Could not set fan speed");
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
