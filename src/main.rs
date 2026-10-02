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

    info!("Starting application...");
    dlr_line_loading_sim::run(stack).await.ok();
}
