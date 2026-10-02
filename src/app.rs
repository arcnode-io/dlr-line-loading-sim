//! Application library for dlr-line-loading-sim.
//!
//! This library reads line loading (amps) and publishes it over MQTT.

#![cfg_attr(not(test), no_std)]

#[cfg(test)]
#[path = "app_test.rs"]
mod app_test;

/// Synthetic line-loading value (amps) -- placeholder until the PZEM-004T
/// current-transformer module lands on the physical unit. Not physically
/// meaningful, just present and varying: a sawtooth over `loop_count`
/// wrapping every 100A.
// Reason: only called from `run()`, which is gated behind hardware features --
// unused (and flagged dead_code) under the plain `--no-default-features`
// unit-test build.
#[allow(dead_code)]
fn synthetic_line_loading_a(loop_count: u32) -> f64 {
    (loop_count % 100) as f64
}

/// MQTT client for publishing sensor data.
pub mod mqtt;
/// WiFi and network stack setup.
pub mod network;

/// System loop rate in seconds.
pub const SYSTEM_RATE: u64 = 2;

/// Application mode - development runs one iteration, beta runs infinite loop.
#[cfg(feature = "integration-test")]
pub const MODE: &str = "development";

/// Application mode - development runs one iteration, beta runs infinite loop.
#[cfg(not(feature = "integration-test"))]
pub const MODE: &str = "beta";

/// Main application function: reads line loading and publishes it to MQTT.
///
/// Initializes the MQTT client and runs the main application loop.
///
/// # Arguments
/// * `stack` - Embassy network stack for MQTT connectivity
///
/// # Returns
/// Ok(()) on successful initialization and first publish (used for testing)
// Reason: the `Result<(), ()>` is a test-observability affordance — dev mode
// returns Ok(()) so a test can `.await` and assert one clean loop. Production
// never returns Err; main.rs discards it with `.ok()`. No real error to type.
#[allow(clippy::result_unit_err)]
#[cfg(all(feature = "rust-mqtt", feature = "embassy-net"))]
pub async fn run(stack: &'static embassy_net::Stack<'static>) -> Result<(), ()> {
    use crate::mqtt::{LINE_LOADING_TOPIC, Mqtt};
    use defmt::info;
    use embassy_time::{Duration, Timer};

    let mut loop_count: u32 = 0;

    loop {
        let mut mqtt_client = match Mqtt::init(stack).await {
            Some(client) => client,
            None => {
                info!("MQTT connect failed -- retrying...");
                Timer::after(Duration::from_secs(SYSTEM_RATE)).await;
                continue;
            }
        };

        info!("Starting application loop ({}s interval)...", SYSTEM_RATE);
        loop {
            loop_count += 1;
            let line_loading_a = synthetic_line_loading_a(loop_count);
            info!("Loop #{}: line_loading = {}A", loop_count, line_loading_a);

            if !mqtt_client
                .publish(LINE_LOADING_TOPIC, line_loading_a)
                .await
            {
                break; // connection broken -- reconnect
            }

            if MODE == "development" {
                return Ok(());
            }

            Timer::after(Duration::from_secs(SYSTEM_RATE)).await;
        }

        info!("MQTT connection lost -- reconnecting...");
        Timer::after(Duration::from_secs(SYSTEM_RATE)).await;
    }
}
