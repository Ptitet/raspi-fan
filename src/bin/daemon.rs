use std::{
    fs,
    io::Read,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use clap::Parser;
use rppal::pwm::{Channel, Polarity, Pwm};
use serde::{Deserialize, Serialize};

use raspi_fan::*;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Daemon {
    #[arg(short, long, value_name = "CONFIG", default_value = DEFAULT_CONFIG_PATH)]
    config: PathBuf,
    #[arg(short, long, value_name = "SOCKET", default_value = DEFAULT_SOCKET)]
    socket: PathBuf,
}

#[derive(Serialize, Deserialize)]
enum FanMode {
    Auto,
    Manual,
}

#[derive(Serialize, Deserialize)]
struct Config {
    mode: FanMode,
    speed: f64,
    /// BCM pin number
    pin: u8,
    pwm_frequency: f64,
    sleep: u64,
    min_speed: f64,
    max_speed: f64,
    min_temp: f64,
    max_temp: f64,
}

impl Config {
    pub fn parse_from(path: &PathBuf) -> Self {
        let config_str = fs::read_to_string(path).expect("Failed to read config");
        toml::from_str(&config_str).expect("Failed to parse config")
    }

    pub fn compute_duty_cycle(&self, cpu_temp: f64) -> f64 {
        let raw_duty_cycle = (cpu_temp - self.min_temp) / (self.max_temp - self.min_temp);
        raw_duty_cycle.clamp(self.min_speed, self.max_speed)
    }

    pub fn pwm_channel(&self) -> Channel {
        match self.pin {
            12 | 18 => Channel::Pwm0,
            13 | 19 => Channel::Pwm1,
            _ => panic!("BCM pin {} is not valid for hardware PWM", self.pin),
        }
    }
}

fn get_cpu_temp() -> f64 {
    fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .expect("Could not read CPU temperature")
        .parse::<f64>()
        .expect("Could not parse CPU temperature")
        / 1000.
}

fn main() {
    let args = Daemon::parse();

    let config = Config::parse_from(&args.config);

    let pwm = Pwm::with_frequency(
        config.pwm_channel(),
        config.pwm_frequency,
        config.min_speed,
        Polarity::Normal,
        true,
    )
    .expect("Could not create PWM instance");

    let _ = fs::remove_file(&args.socket);

    let listener = UnixListener::bind(&args.socket).expect("Failed to create IPC socket");

    let config = Arc::new(Mutex::new(config));

    {
        let config = Arc::clone(&config);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let config = config.lock().expect("Could not acquire config lock");
                let stream = stream.expect("Could not open stream");
                handle_client(stream, config);
            }
        });
    }

    loop {
        let config = config.lock().expect("Could not acquire config lock");
        let cpu_temp = get_cpu_temp();
        let duty_cycle = config.compute_duty_cycle(cpu_temp);
        pwm.set_duty_cycle(duty_cycle)
            .expect("Failed to change duty cycle");
        thread::sleep(Duration::from_secs(config.sleep));
    }
}

fn handle_client(mut stream: UnixStream, _config: MutexGuard<Config>) {
    let mut text = String::new();
    stream
        .read_to_string(&mut text)
        .expect("Could not read ipc message");
    println!("Got a message ! {text}");
}
