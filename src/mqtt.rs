//! MQTT client for publishing sensor data.
//!
//! Uses rust-mqtt for no_std async MQTT v5 client.

/// MQTT topic for publishing a synthetic line-loading value (amps).
///
/// Placeholder until the PZEM-004T + CT module lands on the physical unit --
/// value is synthetic, not a real measurement (see `app::synthetic_line_loading_a`).
/// Bare-topic convention, not ADR-002 shape: this whole subsystem (`mock_derms`
/// in ems/readme.md's deployment diagram) is bounded off from the real ems/
/// device-template contract, so there's no cross-project consistency reason
/// pulling this topic toward that shape.
pub const LINE_LOADING_TOPIC: &str = "test/line_loading/A";

#[cfg(feature = "rust-mqtt")]
use core::fmt::Write;
#[cfg(feature = "rust-mqtt")]
use embassy_net::{Ipv4Address, Stack, tcp::TcpSocket};
#[cfg(feature = "rust-mqtt")]
use embassy_time::Duration;
#[cfg(feature = "rust-mqtt")]
use heapless::String;
#[cfg(feature = "rust-mqtt")]
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::publish_packet::QualityOfService,
    utils::rng_generator::CountingRng,
};

/// MQTT client for publishing sensor data.
#[cfg(all(feature = "rust-mqtt", feature = "esp-wifi"))]
pub struct Mqtt<'a> {
    /// MQTT client instance
    client: MqttClient<'a, TcpSocket<'a>, 5, CountingRng>,
}

#[cfg(all(feature = "rust-mqtt", feature = "esp-wifi"))]
impl<'a> Mqtt<'a> {
    /// Initialize MQTT client and connect to broker.
    ///
    /// # Arguments
    /// * `stack` - Embassy network stack for TCP/IP connectivity
    ///
    /// # Returns
    /// `Some(client)` connected and ready to publish. `None` if the TCP
    /// connect or MQTT handshake failed (e.g. broker still down on a
    /// reconnect attempt) -- caller should back off and retry `init` rather
    /// than treat this as fatal.
    pub async fn init(stack: &'a Stack<'a>) -> Option<Self> {
        // Parse broker config from env vars baked at build time
        let mqtt_host = env!("MQTT_HOST");
        let mqtt_port: u16 = env!("MQTT_PORT").parse().unwrap();

        #[cfg(feature = "defmt")]
        defmt::info!("🚀 Initializing MQTT client...");
        #[cfg(feature = "defmt")]
        defmt::info!("🏠 MQTT broker: {}:{}", mqtt_host, mqtt_port);

        // Create TCP socket buffers (static for 'static lifetime)
        static mut RX_BUFFER: [u8; 4096] = [0; 4096];
        static mut TX_BUFFER: [u8; 4096] = [0; 4096];

        let mut socket = unsafe {
            let rx_buf = &mut *core::ptr::addr_of_mut!(RX_BUFFER);
            let tx_buf = &mut *core::ptr::addr_of_mut!(TX_BUFFER);
            TcpSocket::new(*stack, rx_buf, tx_buf)
        };

        socket.set_timeout(Some(Duration::from_secs(10)));

        // Connect to MQTT broker
        let ip: Ipv4Address = mqtt_host.parse().unwrap();
        let remote_endpoint = (ip, mqtt_port);

        #[cfg(feature = "defmt")]
        defmt::info!(
            "🔗 Attempting TCP connection to MQTT broker at {}:{}",
            mqtt_host,
            mqtt_port
        );
        #[cfg(feature = "defmt")]
        defmt::info!("⏱️  TCP timeout set to 10 seconds");

        if let Err(_e) = socket.connect(remote_endpoint).await {
            #[cfg(feature = "defmt")]
            defmt::info!("❌ TCP connect to broker failed: {:?}", _e);
            return None;
        }

        #[cfg(feature = "defmt")]
        defmt::info!("✅ TCP connected to MQTT broker successfully!");

        // Create MQTT client config
        let mut config = ClientConfig::new(
            rust_mqtt::client::client_config::MqttVersion::MQTTv5,
            CountingRng(20000),
        );
        config.add_client_id("esp32c3");
        config.max_packet_size = 100;

        // Create MQTT client buffers (static for 'static lifetime)
        static mut MQTT_RECV_BUFFER: [u8; 256] = [0; 256];
        static mut MQTT_SEND_BUFFER: [u8; 256] = [0; 256];

        let mut client = unsafe {
            let send_buf = &mut *core::ptr::addr_of_mut!(MQTT_SEND_BUFFER);
            let recv_buf = &mut *core::ptr::addr_of_mut!(MQTT_RECV_BUFFER);
            MqttClient::<_, 5, _>::new(socket, send_buf, 256, recv_buf, 256, config)
        };

        #[cfg(feature = "defmt")]
        defmt::info!("🤝 Attempting MQTT broker handshake...");
        #[cfg(feature = "defmt")]
        defmt::info!("🆔 Client ID: esp32c3");

        if let Err(_e) = client.connect_to_broker().await {
            #[cfg(feature = "defmt")]
            defmt::info!("❌ MQTT handshake failed: {:?}", _e);
            return None;
        }

        #[cfg(feature = "defmt")]
        defmt::info!("✅ MQTT broker connected successfully!");

        Some(Self { client })
    }

    /// Publish a temperature value to the specified topic.
    ///
    /// # Arguments
    /// * `topic` - MQTT topic to publish to
    /// * `value` - Temperature value to publish
    ///
    /// # Returns
    /// `true` on success. `false` means the connection is broken -- the
    /// caller should stop using this client and reconnect via `init`.
    pub async fn publish(&mut self, topic: &str, value: f64) -> bool {
        let mut payload: String<32> = String::new();
        write!(&mut payload, "{:.2}", value).ok();

        #[cfg(feature = "defmt")]
        defmt::info!(
            "📤 Publishing to topic '{}': {} bytes",
            topic,
            payload.len()
        );
        #[cfg(feature = "defmt")]
        defmt::info!("📄 Payload: '{}'", payload.as_str());

        match self
            .client
            .send_message(topic, payload.as_bytes(), QualityOfService::QoS0, false)
            .await
        {
            Ok(_) => {
                #[cfg(feature = "defmt")]
                defmt::info!("✅ Message published successfully");
                true
            }
            Err(e) => {
                #[cfg(feature = "defmt")]
                defmt::info!("❌ Failed to publish message: {:?}", e);
                false
            }
        }
    }
}
