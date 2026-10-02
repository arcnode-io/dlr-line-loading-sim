//! Hardware-in-Loop tests for ESP32 firmware.
//!
//! Tests include:
//! - MQTT integration: device publishes line loading to MQTT broker

mod fixtures;

use fixtures::build::{RELEASE_BIN_PATH, build_release_firmware};
use fixtures::containers::{Container, start_mqtt_broker};
use fixtures::flash::download_and_reset;
use rumqttc::v5::{
    AsyncClient, Event, EventLoop, MqttOptions,
    mqttbytes::{QoS, v5::Packet},
};
use std::error::Error;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Topic where line loading (amps) is published.
const LINE_LOADING_TOPIC: &str = "test/line_loading/A";

/// Maximum time to wait for MQTT message (in seconds).
const MQTT_TIMEOUT_SECS: u64 = 80;

/// Starts a broker, checks `WIFI_PASSWORD` is set, and builds the release
/// firmware against the broker's port.
async fn arrange_broker_and_firmware(start: Instant) -> Result<Container, Box<dyn Error>> {
    let broker = start_mqtt_broker().await?;
    println!(
        "[{:.1}s] broker on port {}",
        start.elapsed().as_secs_f32(),
        broker.port
    );

    if std::env::var("WIFI_PASSWORD").is_err() {
        return Err("WIFI_PASSWORD environment variable must be set".into());
    }

    build_release_firmware(broker.port)?;
    println!("[{:.1}s] firmware built", start.elapsed().as_secs_f32());
    Ok(broker)
}

/// Flashes the release binary to the device and resets it.
fn flash_firmware(start: Instant) -> Result<(), Box<dyn Error>> {
    download_and_reset(RELEASE_BIN_PATH).map_err(|e| -> Box<dyn Error> { e.into() })?;
    println!(
        "[{:.1}s] firmware flashed + device reset",
        start.elapsed().as_secs_f32()
    );
    Ok(())
}

/// Waits for a single Publish packet on `eventloop` and returns its payload
/// as a UTF-8 string, or an error on timeout / decode failure.
async fn wait_for_publish_payload(
    eventloop: &mut EventLoop,
    start: Instant,
) -> Result<String, Box<dyn Error>> {
    let result = timeout(Duration::from_secs(MQTT_TIMEOUT_SECS), async {
        loop {
            let notification = eventloop.poll().await?;
            if let Event::Incoming(Packet::Publish(p)) = notification {
                let payload = String::from_utf8(p.payload.to_vec())
                    .map_err(|_| "Invalid UTF-8 in payload")?;
                println!(
                    "[{:.1}s] received: {}",
                    start.elapsed().as_secs_f32(),
                    payload
                );
                return Ok::<String, Box<dyn Error + Send + Sync>>(payload);
            }
        }
    })
    .await;

    match result {
        Ok(Ok(payload)) => Ok(payload),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!("Test timed out after {MQTT_TIMEOUT_SECS} seconds").into()),
    }
}

#[tokio::test]
async fn test_hil_mqtt_integration() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    // Arrange
    let broker = arrange_broker_and_firmware(start).await?;
    let mqttoptions = MqttOptions::new("hil_test_client", "localhost", broker.port);
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    // Subscribe BEFORE flashing so we don't miss the first message.
    client
        .subscribe(LINE_LOADING_TOPIC, QoS::AtLeastOnce)
        .await?;

    // Act
    flash_firmware(start)?;
    let payload = wait_for_publish_payload(&mut eventloop, start).await?;

    // Assert -- synthetic_line_loading_a is a sawtooth in [0, 100).
    let line_loading_a: f32 = payload
        .parse()
        .map_err(|_| "Payload is not a valid float")?;
    assert!(
        (0.0..100.0).contains(&line_loading_a),
        "line_loading {line_loading_a}A outside the synthetic sawtooth's range"
    );
    println!(
        "[{:.1}s] SUCCESS: valid line_loading {}A",
        start.elapsed().as_secs_f32(),
        line_loading_a
    );
    Ok(())
}
