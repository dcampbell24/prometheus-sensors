# Prometheus Sensors

This captures the readings from the sensors bme-280, mcp-9808, and sht-31.
Then it logs the readings with prometheus and reports them to Weather
Underground.

The weather underground features are optionally enabled via:

```sh
cargo run --release --features weather-underground
```

## Configuration Files

Located in the path that the program is run from or in your home directory:

```sh
bus-path.txt # Holds the path to your i2c driver. Defaults to "/dev/i2c-1".
weather-underground.ron # Holds your weather underground id and upload_key.
```

## Debian

You can run the following commands to install this as a debian package and run
it as a service.

```sh
cargo deb
# Replace the package architecture with whatever it is on your system.
sudo dpkg --install target/debian/prometheus-sensors_0.1.0-1_arm64.deb

# Tell systemd that we want it to look for new things.
sudo systemctl daemon-reload
# Start every time we boot.
sudo systemctl enable sensors.service
# Start it now.
sudo systemctl start sensors.service
```
