use std::path::PathBuf;

use chrono::Utc;
use reqwest::{blocking::Client, Url};
use serde::{Deserialize, Serialize};

use crate::{get_file_contents, BME280, MCP9808, SHT31};

const WEATHER_UNDERGROUND_URL: &str =
    "https://rtupdate.wunderground.com/weatherstation/updateweatherstation.php";

const WEATHER_UNDERGROUND_SECRET: &str = "weather-underground.ron";

pub struct WeatherUnderground {
    http_client: Client,
    secret: WeatherUndergroundSecret,
}

impl WeatherUnderground {
    pub fn init() -> anyhow::Result<Self> {
        let secret = get_file_contents(&PathBuf::from(WEATHER_UNDERGROUND_SECRET))?;
        let secret: WeatherUndergroundSecret = ron::from_str(&secret)?;

        Ok(WeatherUnderground {
            http_client: Client::new(),
            secret,
        })
    }

    pub fn send_data(
        &self,
        bme280: &BME280,
        mcp9808: &MCP9808,
        sht31: &SHT31,
    ) -> anyhow::Result<()> {
        let url = Url::parse_with_params(
            WEATHER_UNDERGROUND_URL,
            &[
                ("action", "updateraw"),
                ("ID", &self.secret.id),
                ("PASSWORD", &self.secret.upload_key),
                // YYYY-MM-DD HH:MM:SS
                (
                    "dateutc",
                    &Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                ),
                // humidity - [% outdoor humidity 0-100%]
                ("humidity", &sht31.humidity.to_string()),
                ("tempf", &sht31.temperature_f.to_string()),
                ("temp2f", &mcp9808.temperature_f.to_string()),
                ("temp3f", &bme280.temperature_f.to_string()),
                // baromin - [barometric pressure atm to inches hg (mercury)]
                ("baromin", &(bme280.pressure * 29.92).to_string()),
                ("realtime", "1"),
                // Frequency in seconds.
                ("rtfreg", "10"),
            ],
        )?;

        let _response = self.http_client.get(url).send()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct WeatherUndergroundSecret {
    id: String,
    upload_key: String,
}
