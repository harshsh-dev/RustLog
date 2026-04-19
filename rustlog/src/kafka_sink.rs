//! Optional Kafka sink (`--features kafka` + librdkafka).

#[cfg(feature = "kafka")]
use anyhow::Context;
use anyhow::Result;

use crate::config::KafkaSection;

#[cfg(feature = "kafka")]
use rdkafka::config::ClientConfig;
#[cfg(feature = "kafka")]
use rdkafka::producer::{FutureProducer, FutureRecord};

/// When `[kafka].enabled = false`, this is a no-op wrapper.
pub struct KafkaPipeline {
    #[cfg(feature = "kafka")]
    inner: Option<KafkaLive>,
}

#[cfg(feature = "kafka")]
struct KafkaLive {
    producer: FutureProducer,
    topic: String,
}

impl KafkaPipeline {
    pub fn from_section(section: &KafkaSection) -> Result<Self> {
        if !section.enabled {
            #[cfg(feature = "kafka")]
            {
                return Ok(Self { inner: None });
            }
            #[cfg(not(feature = "kafka"))]
            {
                return Ok(Self {});
            }
        }
        #[cfg(not(feature = "kafka"))]
        {
            anyhow::bail!(
                "[kafka].enabled is true, but this binary was built without the `kafka` feature. \
                 Rebuild with `cargo build --features kafka` and install librdkafka (e.g. `brew install librdkafka` on macOS)."
            );
        }
        #[cfg(feature = "kafka")]
        {
            Ok(Self {
                inner: Some(KafkaLive::connect(section)?),
            })
        }
    }

    pub async fn publish(&self, line: &str) -> Result<()> {
        #[cfg(feature = "kafka")]
        if let Some(inner) = &self.inner {
            inner.publish(line).await?;
        }
        #[cfg(not(feature = "kafka"))]
        let _ = line;
        Ok(())
    }
}

#[cfg(feature = "kafka")]
impl KafkaLive {
    fn connect(section: &KafkaSection) -> Result<Self> {
        let topic = if section.topic.is_empty() {
            anyhow::bail!("[kafka].topic must be non-empty when Kafka is enabled");
        } else {
            section.topic.clone()
        };
        let brokers = if section.brokers.is_empty() {
            "localhost:9092".to_string()
        } else {
            section.brokers.join(",")
        };
        let mut cfg = ClientConfig::new();
        cfg.set("bootstrap.servers", &brokers);
        cfg.set("message.timeout.ms", "5000");
        if let Some(id) = &section.client_id {
            cfg.set("client.id", id);
        }
        let producer: FutureProducer = cfg
            .create()
            .with_context(|| format!("create Kafka producer for brokers={brokers:?}"))?;
        Ok(Self { producer, topic })
    }

    async fn publish(&self, line: &str) -> Result<()> {
        let record = FutureRecord::to(&self.topic)
            .payload(line.as_bytes())
            .key("");
        self.producer
            .send(record, std::time::Duration::from_secs(5))
            .await
            .map_err(|(e, _)| anyhow::anyhow!("Kafka publish failed: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_is_ok_without_feature() {
        let s = KafkaSection::default();
        assert!(KafkaPipeline::from_section(&s).is_ok());
    }

    #[cfg(not(feature = "kafka"))]
    #[test]
    fn enabled_errors_without_kafka_feature() {
        let s = KafkaSection {
            enabled: true,
            brokers: vec!["localhost:9092".into()],
            topic: "t".into(),
            client_id: None,
        };
        assert!(KafkaPipeline::from_section(&s).is_err());
    }
}
