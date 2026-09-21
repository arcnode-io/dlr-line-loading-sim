use super::*;

#[test]
fn test_synthetic_line_loading_varies_with_loop_count() {
    // Arrange & Act
    let first = synthetic_line_loading_a(0);
    let second = synthetic_line_loading_a(1);

    // Assert
    assert_ne!(first, second);
}

#[test]
fn test_synthetic_line_loading_wraps_within_sawtooth_period() {
    // Arrange & Act
    let actual = synthetic_line_loading_a(150);

    // Assert -- 150 % 100 == 50
    assert_eq!(actual, 50.0);
}
