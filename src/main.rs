use std::time::Instant;

use bme280::i2c;
use embedded_hal::delay::DelayNs;
use linux_embedded_hal::{Delay, I2cdev};
use mcp9808::reg_conf::{Configuration, ShutdownMode};
use mcp9808::reg_res::ResolutionVal;
use mcp9808::reg_temp_generic::ReadableTempRegister;
use metrics::{gauge, Gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use sht31::mode::{Sht31Measure, Sht31Reader, SingleShot};
use sht31::{Accuracy, TemperatureUnit};

const BUS_PATH: &str = "/dev/i2c-1";
const TEMPERATURE_DIFFERENCE: &str = "sensors_temperature_difference_C";
const LOOP_TIMING: &str = "sensors_loop_timing";

const BME280_HUMIDITY: &str = "sensors_humidity_percent_bme280";
const BME280_PRESSURE: &str = "sensors_pressure_atm_bme280";
const BME280_TEMPERATURE_C: &str = "sensors_temperature_celsius_bme280";
const BME280_TEMPERATURE_F: &str = "sensors_temperature_fahrenheit_bme280";

const MCP9808_TEMPERATURE_C: &str = "sensors_temperature_celsius_mcp9808";
const MCP9808_TEMPERATURE_F: &str = "sensors_temperature_fahrenheit_mcp9808";

const SHT31_HUMIDITY: &str = "sensors_humidity_percent_sht31";
const SHT31_TEMPERATURE_C: &str = "sensors_temperature_celsius_sht31";
const SHT31_TEMPERATURE_F: &str = "sensors_temperature_fahrenheit_sht31";

fn main() {
    let builder = PrometheusBuilder::new();
    // Defaults to enabled, listening at 0.0.0.0:9000
    builder
        .install()
        .expect("failed to install recorder/exporter");

    let mut delay = Delay;

    let i2c_bus = I2cdev::new(BUS_PATH).unwrap();
    let mut bme280 = BME280::init(i2c_bus, &mut delay);

    let i2c_bus = I2cdev::new(BUS_PATH).unwrap();
    let mut mcp9808 = MCP9808::init(i2c_bus);

    let i2c_bus = I2cdev::new(BUS_PATH).unwrap();
    let mut sht31 = SHT31::init(i2c_bus, &mut delay);

    let mut temperature_difference = Difference::init();
    let loop_time = gauge!(LOOP_TIMING);

    loop {
        let t0 = Instant::now();
        sht31.measure();
        delay.delay_ms(10_000);

        bme280.read(&mut delay);
        bme280.report_metrics();

        mcp9808.read();
        mcp9808.report_metrics();

        sht31.read();
        sht31.report_metrics();

        temperature_difference.read_temperature_difference(&mut bme280, &mut mcp9808);

        loop_time.set(t0.elapsed().as_secs_f64());
    }
}

// Pressure ± 100 Pa
// Relative Humidity ± 3 %
// Temperature ± 1 °C
pub struct BME280 {
    bme280: i2c::BME280<I2cdev>,

    humidity: f32,
    pressure: f32,
    temperature_c: f32,
    temperature_f: f32,

    metric_humidity: Gauge,
    metric_pressure: Gauge,
    metric_temperature_c: Gauge,
    metric_temperature_f: Gauge,
}

impl BME280 {
    fn init(i2c_bus: I2cdev, delay: &mut Delay) -> Self {
        // Initialize the BME280 using the primary I2C address 0x76.
        let mut bme280 = i2c::BME280::new_primary(i2c_bus);
        bme280.init(delay).unwrap();

        BME280 {
            bme280,

            humidity: 0.0,
            pressure: 0.0,
            temperature_c: 0.0,
            temperature_f: 0.0,

            metric_humidity: gauge!(BME280_HUMIDITY),
            metric_pressure: gauge!(BME280_PRESSURE),
            metric_temperature_c: gauge!(BME280_TEMPERATURE_C),
            metric_temperature_f: gauge!(BME280_TEMPERATURE_F),
        }
    }

    fn read(&mut self, delay: &mut Delay) {
        let measurements = self.bme280.measure(delay).unwrap();

        self.humidity = measurements.humidity;
        self.pressure = measurements.pressure * 0.000_009_87;
        self.temperature_c = measurements.temperature;
        self.temperature_f = (self.temperature_c * 1.8) + 32.0;
    }

    fn report_metrics(&mut self) {
        self.metric_humidity.set(self.humidity);
        self.metric_pressure.set(self.pressure);
        self.metric_temperature_c.set(self.temperature_c);
        self.metric_temperature_f.set(self.temperature_f);
    }
}

// Accuracy: Typical ±0.25°C /  Maximum ±0.5°C
pub struct MCP9808 {
    mcp9808: mcp9808::MCP9808<I2cdev>,

    temperature_c: f32,
    temperature_f: f32,

    metric_temperature_c: Gauge,
    metric_temperature_f: Gauge,
}

impl MCP9808 {
    fn init(i2c_bus: I2cdev) -> Self {
        let mut mcp9808 = mcp9808::MCP9808::new(i2c_bus);
        let mut conf = mcp9808.read_configuration().unwrap();
        conf.set_shutdown_mode(ShutdownMode::Continuous);
        mcp9808.write_register(conf).unwrap();

        MCP9808 {
            mcp9808,

            temperature_c: 0.0,
            temperature_f: 0.0,

            metric_temperature_c: gauge!(MCP9808_TEMPERATURE_C),
            metric_temperature_f: gauge!(MCP9808_TEMPERATURE_F),
        }
    }

    fn read(&mut self) {
        let temperature = self.mcp9808.read_temperature().unwrap();
        self.temperature_c = temperature.get_celsius(ResolutionVal::Deg_0_0625C);
        self.temperature_f = (self.temperature_c * 1.8) + 32.0;
    }

    fn report_metrics(&mut self) {
        self.metric_temperature_c.set(self.temperature_c);
        self.metric_temperature_f.set(self.temperature_f);
    }
}

// Relative Humidity ± 2 %
// Temperature ± 0.3 °C
pub struct SHT31 {
    sht31: sht31::SHT31<SingleShot, I2cdev>,

    humidity: f32,
    temperature_c: f32,
    temperature_f: f32,

    metric_humidity: Gauge,
    metric_temperature_c: Gauge,
    metric_temperature_f: Gauge,
}

impl SHT31 {
    fn init(i2c_bus: I2cdev, delay: &mut Delay) -> Self {
        let sht31 = sht31::SHT31::new(i2c_bus, delay)
            .with_mode(SingleShot::new())
            .with_accuracy(Accuracy::High)
            .with_unit(TemperatureUnit::Celsius);

        SHT31 {
            sht31,

            humidity: 0.0,
            temperature_c: 0.0,
            temperature_f: 0.0,

            metric_humidity: gauge!(SHT31_HUMIDITY),
            metric_temperature_c: gauge!(SHT31_TEMPERATURE_C),
            metric_temperature_f: gauge!(SHT31_TEMPERATURE_F),
        }
    }

    fn measure(&mut self) {
        self.sht31.measure().unwrap();
    }

    fn read(&mut self) {
        let reading = self.sht31.read().unwrap();
        self.humidity = reading.humidity;
        self.temperature_c = reading.temperature;
        self.temperature_f = (self.temperature_c * 1.8) + 32.0;
    }

    fn report_metrics(&mut self) {
        self.metric_humidity.set(self.humidity);
        self.metric_temperature_c.set(self.temperature_c);
        self.metric_temperature_f.set(self.temperature_f);
    }


}

struct Difference {
    temperature_difference: Gauge,
}

impl Difference {
    fn init() -> Self {
        let temperature_difference = gauge!(TEMPERATURE_DIFFERENCE);
        Difference {
            temperature_difference,
        }
    }

    fn read_temperature_difference(&mut self, bme280: &mut BME280, mcp9808: &mut MCP9808) {
        self.temperature_difference
            .set(bme280.temperature_c - mcp9808.temperature_c);
    }
}
