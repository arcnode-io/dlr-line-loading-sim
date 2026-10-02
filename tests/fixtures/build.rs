//! Firmware build helper shared by HIL tests.

use std::error::Error;
use std::net::UdpSocket;
use std::process::Command;

/// Full feature set required to build the production firmware binary.
const RELEASE_FEATURES: &str = "defmt,esp-hal,embassy-time,esp-hal-embassy,rtt-target,esp-alloc,embassy-executor,embassy-net,esp-bootloader-esp-idf,critical-section,esp-wifi,smoltcp,static_cell,rust-mqtt,heapless";

/// Path to the built release binary, relative to the crate root.
pub const RELEASE_BIN_PATH: &str =
    "target/riscv32imc-unknown-none-elf/release/dlr-line-loading-sim";

/// Finds this machine's LAN IP the way the device would reach it -- the
/// route the OS picks toward a public address, not `cfg.yml`'s value (which
/// drifts with DHCP). Same trick `dlr-rtu-firmware/tests/test_hil.py`
/// uses. No packets actually leave the machine (UDP connect just picks a route).
fn local_lan_ip() -> Result<String, Box<dyn Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    Ok(socket.local_addr()?.ip().to_string())
}

/// Builds the release firmware binary, baking in the given MQTT broker port
/// and this machine's actual LAN IP as the broker host.
pub fn build_release_firmware(mqtt_port: u16) -> Result<(), Box<dyn Error>> {
    let mqtt_host = local_lan_ip()?;
    println!("MQTT_HOST for firmware build: {mqtt_host}");

    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--bin=dlr-line-loading-sim")
        .arg(format!("--features={RELEASE_FEATURES}"))
        .env("MQTT_HOST", mqtt_host)
        .env("MQTT_PORT", mqtt_port.to_string())
        .status()?;

    if !status.success() {
        return Err("firmware build failed".into());
    }
    Ok(())
}
