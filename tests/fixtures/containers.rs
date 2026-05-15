//! Testcontainer fixtures with dynamic port allocation.

use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::{ContainerAsync, GenericImage, ImageExt, runners::AsyncRunner};

/// Connection info for a running testcontainer.
///
/// Container stops automatically when this struct is dropped.
pub struct Container {
    /// Container host (always localhost).
    pub host: String,
    /// Dynamic mapped port.
    pub port: u16,
    /// Pre-built connection URL.
    pub url: String,
    /// Reason: held to keep container alive via RAII.
    _inner: ContainerAsync<GenericImage>,
}

/// Internal building block — start any Docker image with a single exposed port.
///
/// Uses `with_mapped_port(0, ...)` (OS-assigned host port, single binding)
/// rather than `with_exposed_port` because the latter triggers
/// testcontainers-rs's `publish_all_ports = true` branch — docker `-P`
/// mode publishes EVERY image-EXPOSE port, multiplying collision odds
/// on a busy CI runner (e.g. HiveMQ exposes 1883/8000/8083/8443/8883).
async fn start_container(
    image: &str,
    tag: &str,
    port: u16,
    wait_for_log: &str,
) -> Result<Container, Box<dyn std::error::Error>> {
    let inner = GenericImage::new(image, tag)
        .with_wait_for(WaitFor::message_on_stdout(wait_for_log))
        .with_mapped_port(0, port.tcp())
        .start()
        .await?;

    let mapped = inner.get_host_port_ipv4(port).await?;

    Ok(Container {
        host: "localhost".to_string(),
        port: mapped,
        url: format!("http://localhost:{mapped}"),
        _inner: inner,
    })
}

/// Start an HiveMQ MQTT broker with dynamic port.
///
/// # Example
/// ```rust
/// let broker = start_mqtt_broker().await?;
/// let opts = MqttOptions::new("client", &broker.host, broker.port);
/// ```
pub async fn start_mqtt_broker() -> Result<Container, Box<dyn std::error::Error>> {
    start_container(
        "hivemq/hivemq-ce",
        "latest",
        1883,
        "Started TCP Listener on address 0.0.0.0 and on port 1883.",
    )
    .await
}
