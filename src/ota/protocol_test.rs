//! Unit tests for drogue-ajour protocol serialization.

#[cfg(test)]
mod tests {
    use drogue_ajour_protocol::{Command, Status};

    #[test]
    fn test_status_first_serializes() {
        // Arrange
        let status = Status::first(b"1.0.0", Some(4096), None);

        // Act
        let bytes = serde_cbor::to_vec(&status).unwrap();

        // Assert
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_command_sync_deserializes() {
        // Arrange
        let cmd = Command::new_sync(b"1.0.0", Some(300), None);
        let bytes = serde_cbor::to_vec(&cmd).unwrap();

        // Act
        let decoded: Command = serde_cbor::from_slice(&bytes).unwrap();

        // Assert
        assert!(matches!(decoded, Command::Sync { .. }));
    }

    #[test]
    fn test_command_write_contains_data() {
        // Arrange
        let firmware_chunk = vec![0xAB; 4096];
        let cmd = Command::new_write(b"2.0.0", 0, &firmware_chunk, None);
        let bytes = serde_cbor::to_vec(&cmd).unwrap();

        // Act
        let decoded: Command = serde_cbor::from_slice(&bytes).unwrap();

        // Assert
        match decoded {
            Command::Write { offset, data, .. } => {
                assert_eq!(offset, 0);
                assert_eq!(data.len(), 4096);
            }
            _ => panic!("Expected Write command"),
        }
    }
}
