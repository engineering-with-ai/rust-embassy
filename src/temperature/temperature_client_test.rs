use super::temperature_client::celsius_to_fahrenheit;

#[test]
fn test_celsius_to_fahrenheit() {
    // Arrange & Act & Assert
    assert_eq!(celsius_to_fahrenheit(0.0), 32.0);
    assert_eq!(celsius_to_fahrenheit(100.0), 212.0);
}
