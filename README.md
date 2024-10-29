# Prometheus Sensors

This captures the readings from the sensors bme-280, mcp-9808, and sht-31.
Then it logs the readings with prometheus and reports them to Weather
Underground.

You have to provide the Weather Underground credentials and the BUS_PATH may
be different from what is provided.

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
