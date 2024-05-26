use bme280::i2c;
use embedded_hal::delay::DelayNs;
use linux_embedded_hal::{Delay, I2cdev};
use mcp9808::reg_conf::{Configuration, ShutdownMode};
use mcp9808::reg_res::ResolutionVal;
use mcp9808::reg_temp_generic::ReadableTempRegister;
use metrics::{gauge, Gauge};
use metrics_exporter_prometheus::PrometheusBuilder;

const BUS_PATH: &str = "/dev/i2c-1";
const TEMPERATURE_DIFFERENCE: &str = "temperature_difference_C";

const BME280_HUMIDITY: &str = "humidity_percent";
const BME280_PRESSURE: &str = "pressure_atm";
const BME280_TEMPERATURE_C: &str = "temperature_C_";
const BME280_TEMPERATURE_F: &str = "temperature_F_";

const MCP9808_TEMPERATURE_C: &str = "temperature_C";
const MCP9808_TEMPERATURE_F: &str = "temperature_F";

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

    let mut temperature_difference = Difference::init();

    loop {
        bme280.measure(&mut delay);
        mcp9808.read_temperature();
        temperature_difference.read_temperature_difference(&mut bme280, &mut mcp9808);
        delay.delay_ms(1_000);
    }
}

// Pressure ± 100 Pa
// Relative Humidity ± 3 %
// Temperature ± 1 °C
pub struct BME280 {
    bme280: i2c::BME280<I2cdev>,
    humidity: Gauge,
    pressure: Gauge,
    temperature_c_: f32,
    temperature_c: Gauge,
    temperature_f: Gauge,
}

impl BME280 {
    fn init(i2c_bus: I2cdev, delay: &mut Delay) -> Self {
        let humidity = gauge!(BME280_HUMIDITY);
        let pressure = gauge!(BME280_PRESSURE);
        let temperature_c = gauge!(BME280_TEMPERATURE_C);
        let temperature_f = gauge!(BME280_TEMPERATURE_F);

        // Initialize the BME280 using the primary I2C address 0x76.
        let mut bme280 = i2c::BME280::new_primary(i2c_bus);
        bme280.init(delay).unwrap();

        BME280 {
            temperature_c_: 0.0,
            bme280,
            humidity,
            pressure,
            temperature_c,
            temperature_f,
        }
    }

    fn measure(&mut self, delay: &mut Delay) {
        let measurements = self.bme280.measure(delay).unwrap();

        self.humidity.set(measurements.humidity);
        self.pressure.set(measurements.pressure * 0.000_009_87);
        self.temperature_c_ = measurements.temperature;
        self.temperature_c.set(measurements.temperature);
        self.temperature_f
            .set((measurements.temperature * 1.8) + 32.0);
    }
}

// Accuracy: Typical ±0.25°C /  Maximum ±0.5°C
pub struct MCP9808 {
    mcp9808: mcp9808::MCP9808<I2cdev>,
    temperature_c_: f32,
    temperature_c: Gauge,
    temperature_f: Gauge,
}

impl MCP9808 {
    fn init(i2c_bus: I2cdev) -> Self {
        let mut mcp9808 = mcp9808::MCP9808::new(i2c_bus);

        let mut conf = mcp9808.read_configuration().unwrap();
        conf.set_shutdown_mode(ShutdownMode::Continuous);
        mcp9808.write_register(conf).unwrap();

        let temperature_c = gauge!(MCP9808_TEMPERATURE_C);
        let temperature_f = gauge!(MCP9808_TEMPERATURE_F);

        MCP9808 {
            temperature_c_: 0.0,
            mcp9808,
            temperature_c,
            temperature_f,
        }
    }

    fn read_temperature(&mut self) {
        let temperature = self.mcp9808.read_temperature().unwrap();
        let temperature = temperature.get_celsius(ResolutionVal::Deg_0_0625C);
        self.temperature_c_ = temperature;
        self.temperature_c.set(temperature);
        self.temperature_f.set((temperature * 1.8) + 32.0);
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
            .set(bme280.temperature_c_ - mcp9808.temperature_c_);
    }
}
