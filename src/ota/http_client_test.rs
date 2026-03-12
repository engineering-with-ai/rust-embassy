//! Unit tests for OTA HTTP client.

#[cfg(test)]
mod tests {
    use super::super::http_client::OtaHttpClient;

    #[test]
    fn test_parse_url_http() {
        // Arrange
        let url = "http://192.168.1.100:8000/firmware.bin";

        // Act
        let result = OtaHttpClient::parse_url(url);

        // Assert
        assert!(result.is_ok());
        let (host, path, use_tls) = result.unwrap();
        assert_eq!(host, "192.168.1.100:8000");
        assert_eq!(path, "/firmware.bin");
        assert!(!use_tls);
    }

    #[test]
    fn test_parse_url_https() {
        // Arrange
        let url = "https://example.com/ota/firmware.bin";

        // Act
        let result = OtaHttpClient::parse_url(url);

        // Assert
        assert!(result.is_ok());
        let (host, path, use_tls) = result.unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(path, "/ota/firmware.bin");
        assert!(use_tls);
    }

    #[test]
    fn test_parse_url_no_path() {
        // Arrange
        let url = "http://192.168.1.1";

        // Act
        let result = OtaHttpClient::parse_url(url);

        // Assert
        assert!(result.is_ok());
        let (host, path, use_tls) = result.unwrap();
        assert_eq!(host, "192.168.1.1");
        assert_eq!(path, "/");
        assert!(!use_tls);
    }

    #[test]
    fn test_parse_url_invalid_scheme() {
        // Arrange
        let url = "ftp://example.com/file.bin";

        // Act
        let result = OtaHttpClient::parse_url(url);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_url_no_scheme() {
        // Arrange
        let url = "example.com/file.bin";

        // Act
        let result = OtaHttpClient::parse_url(url);

        // Assert
        assert!(result.is_err());
    }
}
