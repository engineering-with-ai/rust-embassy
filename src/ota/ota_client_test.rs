//! Unit tests for OTA client.

#[cfg(test)]
mod tests {
    use super::super::ota_client::OtaClient;

    #[test]
    fn test_ota_client_new() {
        // Arrange & Act
        let client = OtaClient::new();

        // Assert
        // Client should be successfully created (compile-time check)
        drop(client);
    }

    #[test]
    fn test_ota_client_default() {
        // Arrange & Act
        let client = OtaClient::default();

        // Assert
        // Default should create valid client
        drop(client);
    }

    #[cfg(feature = "rollback")]
    #[test]
    fn test_rollback_available_returns_false() {
        // Arrange
        let client = OtaClient::new();

        // Act
        let available = client.rollback_available();

        // Assert
        assert!(!available, "Rollback should not be available yet");
    }
}
