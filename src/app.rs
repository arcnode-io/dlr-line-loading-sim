//! Application library for dlr-line-loading-sim.
//!
//! This library provides temperature monitoring and publishing.

#![cfg_attr(not(test), no_std)]

#[cfg(test)]
#[path = "app_test.rs"]
mod app_test;

#[cfg(all(feature = "shtcx", feature = "rust-mqtt", feature = "embassy-net"))]
use crate::temperature::temperature_client::TemperatureClient;

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
/// DLR-rating-to-tap-position control logic.
pub mod tap_control;
/// Temperature sensor client.
pub mod temperature;

/// System loop rate in seconds.
pub const SYSTEM_RATE: u64 = 2;

/// Application mode - development runs one iteration, beta runs infinite loop.
#[cfg(feature = "integration-test")]
pub const MODE: &str = "development";

/// Application mode - development runs one iteration, beta runs infinite loop.
#[cfg(not(feature = "integration-test"))]
pub const MODE: &str = "beta";

/// Main application function that reads temperature and publishes to MQTT.
///
/// Initializes client and runs main application loop.
///
/// # Arguments
/// * `i2c` - I2C bus interface for temperature sensor
/// * `stack` - Embassy network stack for MQTT connectivity
///
/// # Returns
/// Ok(()) on successful initialization and first read (used for testing)
// Reason: the `Result<(), ()>` is a test-observability affordance — dev mode
// returns Ok(()) so a test can `.await` and assert one clean loop. Production
// never returns Err; main.rs discards it with `.ok()`. No real error to type.
#[allow(clippy::result_unit_err)]
#[cfg(all(feature = "shtcx", feature = "rust-mqtt", feature = "embassy-net"))]
pub async fn run<I2C>(i2c: I2C, stack: &'static embassy_net::Stack<'static>) -> Result<(), ()>
where
    I2C: embedded_hal::i2c::I2c,
{
    use crate::mqtt::{LINE_LOADING_TOPIC, MQTT_TOPIC, Mqtt, RATING_TOPIC, TAP_POSITION_TOPIC};
    use crate::tap_control::TapController;
    use defmt::info;
    use embassy_time::{Duration, Timer};

    // Reason: temp_client + tap_controller live outside the reconnect loop --
    // a dropped MQTT connection doesn't change the physical tap position, so
    // resetting tap_controller on reconnect would produce spurious tap
    // "changes" caused by network blips instead of real rating changes.
    let mut temp_client = TemperatureClient::new(i2c);
    let mut tap_controller = TapController::new();
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
        if !mqtt_client.subscribe(RATING_TOPIC).await {
            Timer::after(Duration::from_secs(SYSTEM_RATE)).await;
            continue;
        }

        info!("Starting application loop ({}s interval)...", SYSTEM_RATE);
        loop {
            loop_count += 1;
            let temp_f = temp_client.read_fahrenheit();
            info!("Loop #{}: Temperature = {}F", loop_count, temp_f);

            if !mqtt_client.publish(MQTT_TOPIC, temp_f).await {
                break; // connection broken -- reconnect
            }

            let line_loading_a = synthetic_line_loading_a(loop_count);
            if !mqtt_client
                .publish(LINE_LOADING_TOPIC, line_loading_a)
                .await
            {
                break; // connection broken -- reconnect
            }

            match mqtt_client.try_receive_rating().await {
                Ok(Some(rating_a)) => {
                    let tap = tap_controller.on_rating(rating_a);
                    info!("Rating {}A -> {}", rating_a, tap.as_str());
                    if !mqtt_client
                        .publish_str(TAP_POSITION_TOPIC, tap.as_str())
                        .await
                    {
                        break; // connection broken -- reconnect
                    }
                }
                Ok(None) => {}
                Err(crate::mqtt::ConnectionLost) => break, // connection broken -- reconnect
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
