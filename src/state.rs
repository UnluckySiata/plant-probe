use core::fmt::Write;
use heapless::String;

#[derive(Clone, Copy)]
pub enum Sensor {
    Temperature,
    Light,
    Humidity,
}

const ADC_MAX: u16 = 4096;

impl Sensor {
    fn next(&self) -> Self {
        match self {
            Self::Temperature => Self::Light,
            Self::Light => Self::Humidity,
            Self::Humidity => Self::Temperature,
        }
    }
}

pub struct SensorConfig {
    min: f32,
    max: f32,
}

impl SensorConfig {
    fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }
}

#[derive(Clone, Copy)]
enum Variant {
    Min,
    Max,
}

enum Task {
    Measuring,
    Configuring(Sensor, Variant),
}

pub struct State {
    task: Task,
    out_str: String<1024>,
    temp: f32,
    light: f32,
    humidity: f32,
    temp_cfg: SensorConfig,
    light_cfg: SensorConfig,
    humidity_cfg: SensorConfig,
}

impl State {
    pub fn new() -> Self {
        Self {
            task: Task::Measuring,
            out_str: String::new(),
            temp: 0.0f32,
            light: 0.0f32,
            humidity: 0.0f32,
            temp_cfg: SensorConfig::new(15f32, 40f32),
            light_cfg: SensorConfig::new(0.0f32, 0.7f32),
            humidity_cfg: SensorConfig::new(0.0f32, 0.7f32),
        }
    }

    pub fn is_measuring(&self) -> bool {
        match self.task {
            Task::Measuring => true,
            _ => false,
        }
    }

    pub fn bad_level(&self) -> bool {
        if self.temp < self.temp_cfg.min
            || self.light < self.light_cfg.min
            || self.humidity < self.humidity_cfg.min
            || self.temp > self.temp_cfg.max
            || self.light > self.light_cfg.max
            || self.humidity > self.humidity_cfg.max
        {
            return true;
        }
        return false;
    }

    pub fn update_config(&mut self, level: u16) {
        let ratio = adc_ratio(level, false);
        match self.task {
            Task::Configuring(Sensor::Temperature, Variant::Min) => {
                self.temp_cfg.min = ratio * 100f32;
            }
            Task::Configuring(Sensor::Temperature, Variant::Max) => {
                self.temp_cfg.max = ratio * 100f32;
            }
            Task::Configuring(Sensor::Light, Variant::Min) => {
                self.light_cfg.min = ratio * 1f32;
            }
            Task::Configuring(Sensor::Light, Variant::Max) => {
                self.light_cfg.max = ratio * 1f32;
            }
            Task::Configuring(Sensor::Humidity, Variant::Min) => {
                self.humidity_cfg.min = ratio * 1f32;
            }
            Task::Configuring(Sensor::Humidity, Variant::Max) => {
                self.humidity_cfg.max = ratio * 1f32;
            }
            _ => {}
        }
    }

    pub fn update_measurements(&mut self, temp: f32, light: u16, humidity: u16) {
        self.temp = temp;
        self.light = adc_ratio(light, false);
        self.humidity = adc_ratio(humidity, true);
    }

    pub fn progress(&mut self) {
        self.task = match self.task {
            Task::Measuring => Task::Measuring,
            Task::Configuring(sensor, Variant::Min) => Task::Configuring(sensor, Variant::Max),
            Task::Configuring(sensor, Variant::Max) => {
                Task::Configuring(sensor.next(), Variant::Min)
            }
        }
    }

    pub fn switch(&mut self) {
        self.task = match self.task {
            Task::Measuring => Task::Configuring(Sensor::Temperature, Variant::Min),
            Task::Configuring(_, _) => Task::Measuring,
        }
    }

    pub fn get_repr(&mut self) -> &str {
        self.out_str.clear();

        match self.task {
            Task::Measuring => {
                self.out_str.push_str("MEASURING\n\n").unwrap();
                writeln!(
                    &mut self.out_str,
                    "         Value\n\
                    Temp     {:.2}\n\
                    Light    {:.2}\n\
                    Humidity {:.2}\
                    ",
                    self.temp, self.light, self.humidity,
                )
                .unwrap();
            }
            Task::Configuring(sensor, variant) => {

                let chars = match (sensor, variant) {
                    (Sensor::Temperature, Variant::Min) => [" ", "", ">", "", " ", "", " ", ""],
                    (Sensor::Temperature, Variant::Max) => ["", " ", "", ">", "", " ", "", " "],
                    (Sensor::Light, Variant::Min) => [" ", "", " ", "", ">", "", " ", ""],
                    (Sensor::Light, Variant::Max) => ["", " ", "", " ", "", ">", "", " "],
                    (Sensor::Humidity, Variant::Min) => [" ", "", " ", "", " ", "", ">", ""],
                    (Sensor::Humidity, Variant::Max) => ["", " ", "", " ", "", " ", "", ">"],
                };

                self.out_str.push_str("CONFIGURING\n\n").unwrap();
                writeln!(
                    &mut self.out_str,
                    "          {}Min    {}Max\n\
                    Temp     {}{:5.2} {}{:5.2}\n\
                    Light    {}{:5.2} {}{:5.2}\n\
                    Humidity {}{:5.2} {}{:5.2}\
                    ",
                    chars[0],
                    chars[1],
                    chars[2],
                    self.temp_cfg.min,
                    chars[3],
                    self.temp_cfg.max,
                    chars[4],
                    self.light_cfg.min,
                    chars[5],
                    self.light_cfg.max,
                    chars[6],
                    self.humidity_cfg.min,
                    chars[7],
                    self.humidity_cfg.max,
                )
                .unwrap();
            }
        }

        return &self.out_str;
    }
}

fn adc_ratio(raw_read: u16, inversed: bool) -> f32 {
    let measurement = match inversed {
        true => ADC_MAX - raw_read,
        false => raw_read,
    };
    measurement as f32 / ADC_MAX as f32
}
