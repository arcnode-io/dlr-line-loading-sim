//! ESP32-C3 Embassy firmware application.
//!
//! Main binary for ESP32-C3 with Embassy async runtime, WiFi, and MQTT support.

#![no_std]
#![no_main]
#![allow(missing_docs)]

use defmt::info;
use embassy_executor::Spawner;
use esp_hal::clock::CpuClock;

/// Panic handler for embedded no_std environment.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

/// Main firmware entry point.
#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();

    info!("=== ESP32-C3 Firmware Starting ===");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    info!("Connecting to WiFi...");
    let stack = dlr_line_loading_sim::network::setup_network(&spawner, peripherals).await;
    info!("WiFi connected!");

    // Reason: setup_network consumes peripherals, steal back for I2C (GPIO10/GPIO8 unused by WiFi)
    let peripherals = unsafe { esp_hal::peripherals::Peripherals::steal() };
    let i2c =
        esp_hal::i2c::master::I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
            .unwrap()
            .with_sda(peripherals.GPIO10)
            .with_scl(peripherals.GPIO8);

    info!("Hardware initialized. Starting application...");
    dlr_line_loading_sim::run(i2c, stack).await.ok();
}
