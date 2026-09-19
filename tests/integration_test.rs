//! Integration tests. Validates sensor hardware. Uses embedded-test  to run on-device tests.

#![no_std]
#![no_main]

#[allow(missing_docs)]
mod _app_desc {
    esp_bootloader_esp_idf::esp_app_desc!();
}

#[cfg(test)]
#[embedded_test::tests(executor = esp_hal_embassy::Executor::new())]
mod integration_tests {
    use esp_hal::timer::systimer::SystemTimer;

    /// Initializes the test environment.
    ///
    /// Sets up ESP32-C3 peripherals, Embassy executor, and defmt logging via RTT.
    #[init]
    fn init() {
        let peripherals = esp_hal::init(esp_hal::Config::default());

        let timer0 = SystemTimer::new(peripherals.SYSTIMER);
        esp_hal_embassy::init(timer0.alarm0);

        rtt_target::rtt_init_defmt!();
    }

    /// Test temperature sensor reads valid Fahrenheit values.
    ///
    /// Validates Temperature sensor can communicate and return reasonable temperature readings.
    /// Checks that readings are within expected range and consistent across multiple reads.
    #[test]
    async fn test_temperature_sensor() {
        use esp_hal::i2c::master::I2c;

        // Safety: Peripherals already initialized in init(), stealing for test use
        let peripherals = unsafe { esp_hal::peripherals::Peripherals::steal() };

        // Initialize I2C for SHTC3 (SDA=GPIO10, SCL=GPIO8)
        let i2c = I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
            .expect("Failed to initialize I2C")
            .with_sda(peripherals.GPIO10)
            .with_scl(peripherals.GPIO8);

        let mut client = dlr_tap_regulator_sim::temperature::temperature_client::TemperatureClient::new(i2c);

        // Act - Read temperature
        let fahrenheit = client.read_fahrenheit();

        // Assert - Temperature is in reasonable range (50-104°F)
        assert!(fahrenheit >= 50.0 && fahrenheit <= 104.0);

        // Act - Read again for consistency check
        let fahrenheit2 = client.read_fahrenheit();

        // Assert - Readings are consistent (within 9°F tolerance)
        let diff = (fahrenheit - fahrenheit2).abs();
        assert!(diff < 9.0);
    }
}
