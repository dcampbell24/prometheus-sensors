use bme280::i2c;
use embedded_hal::delay::DelayNs;
use linux_embedded_hal::{Delay, I2cdev};
use metrics::{gauge, Gauge};
use metrics_exporter_prometheus::PrometheusBuilder;

fn main() {
    let builder = PrometheusBuilder::new();
    // Defaults to enabled, listening at 0.0.0.0:9000
    builder
        .install()
        .expect("failed to install recorder/exporter");

    let mut delay = Delay;
    let mut bme280 = BME280::init(&mut delay);
    loop {
        bme280.measure(&mut delay);
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
    temperature_c: Gauge,
    temperature_f: Gauge,
}

impl BME280 {
    fn init(delay: &mut Delay) -> Self {
        let humidity = gauge!("humidity_percent");
        let pressure = gauge!("pressure_atm");
        let temperature_c = gauge!("temperature_celsius");
        let temperature_f = gauge!("temperature_fahrenheit");

        let i2c_bus = I2cdev::new("/dev/i2c-1").unwrap();
        // Initialize the BME280 using the primary I2C address 0x76.
        let mut bme280 = i2c::BME280::new_primary(i2c_bus);
        bme280.init(delay).unwrap();

        BME280 {
            bme280, humidity, pressure, temperature_c, temperature_f
        }
    }

    fn measure(&mut self, delay: &mut Delay) {
        let measurements = self.bme280.measure(delay).unwrap();

        self.humidity.set(measurements.humidity);
        self.pressure.set(measurements.pressure * 0.000_009_87);
        self.temperature_c.set(measurements.temperature);
        self.temperature_f.set((measurements.temperature * 1.8) + 32.0);
    }
}
